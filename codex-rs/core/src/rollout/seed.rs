use std::io;
use std::path::Path;

use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::RolloutItem;
use codex_protocol::protocol::RolloutLine;
use tracing::warn;

use crate::protocol::EventMsg;

pub(crate) async fn load_rollout_lines(path: &Path) -> io::Result<Vec<RolloutLine>> {
    let text = tokio::fs::read_to_string(path).await?;
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut lines = Vec::new();
    for raw in text.lines() {
        if raw.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<RolloutLine>(raw) {
            Ok(line) => lines.push(line),
            Err(err) => warn!("failed to parse rollout line as JSON: err={err} line={raw:?}"),
        }
    }
    Ok(lines)
}

pub(crate) fn extract_between_last_two_context_compactions(
    lines: &[RolloutLine],
) -> Vec<RolloutLine> {
    let compacted_positions: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter_map(|(idx, line)| match &line.item {
            RolloutItem::EventMsg(EventMsg::ContextCompacted(_)) => Some(idx),
            _ => None,
        })
        .collect();

    if compacted_positions.is_empty() {
        return Vec::new();
    }

    let (start, end) = if compacted_positions.len() == 1 {
        (0, compacted_positions[0])
    } else {
        let Some(&end) = compacted_positions.last() else {
            return Vec::new();
        };
        let start = compacted_positions[compacted_positions.len() - 2].saturating_add(1);
        (start, end)
    };

    if start >= end || start >= lines.len() {
        return Vec::new();
    }

    lines[start..end].to_vec()
}

pub(crate) fn sanitize_for_seed(lines: &[RolloutLine]) -> Vec<RolloutLine> {
    let mut cleaned = Vec::new();
    for line in lines {
        let RolloutItem::ResponseItem(item) = &line.item else {
            continue;
        };

        let Some(item) = sanitize_response_item(item) else {
            continue;
        };

        cleaned.push(RolloutLine {
            timestamp: line.timestamp.clone(),
            item: RolloutItem::ResponseItem(item),
        });
    }
    cleaned
}

pub(crate) async fn write_rollout_lines_jsonl(
    path: &Path,
    lines: &[RolloutLine],
) -> io::Result<()> {
    let mut out = String::new();
    for line in lines {
        let json =
            serde_json::to_string(line).map_err(|e| io::Error::other(format!("json: {e}")))?;
        out.push_str(&json);
        out.push('\n');
    }
    tokio::fs::write(path, out).await
}

fn sanitize_response_item(item: &ResponseItem) -> Option<ResponseItem> {
    match item {
        ResponseItem::Reasoning { .. } | ResponseItem::Compaction { .. } | ResponseItem::Other => {
            None
        }
        ResponseItem::FunctionCallOutput { call_id, output } => {
            let mut output = output.clone();
            output.content = strip_tool_output_telemetry(&output.content);
            Some(ResponseItem::FunctionCallOutput {
                call_id: call_id.clone(),
                output,
            })
        }
        ResponseItem::CustomToolCallOutput { call_id, output } => {
            Some(ResponseItem::CustomToolCallOutput {
                call_id: call_id.clone(),
                output: strip_tool_output_telemetry(output),
            })
        }
        other => Some(other.clone()),
    }
}

fn strip_tool_output_telemetry(output: &str) -> String {
    let mut kept = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim_end();
        if trimmed.starts_with("Chunk ID:")
            || trimmed.starts_with("Wall time:")
            || trimmed.starts_with("Original token count:")
            || trimmed.starts_with("Total output lines:")
        {
            continue;
        }
        kept.push(trimmed);
    }
    kept.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::models::ContentItem;
    use codex_protocol::models::FunctionCallOutputPayload;
    use codex_protocol::protocol::ContextCompactedEvent;

    fn user_message(text: &str) -> ResponseItem {
        ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![ContentItem::InputText {
                text: text.to_string(),
            }],
        }
    }

    #[test]
    fn extracts_last_segment_between_context_compactions() {
        let lines = vec![
            RolloutLine {
                timestamp: "t0".to_string(),
                item: RolloutItem::ResponseItem(user_message("before")),
            },
            RolloutLine {
                timestamp: "t1".to_string(),
                item: RolloutItem::EventMsg(EventMsg::ContextCompacted(ContextCompactedEvent {})),
            },
            RolloutLine {
                timestamp: "t2".to_string(),
                item: RolloutItem::ResponseItem(user_message("keep-1")),
            },
            RolloutLine {
                timestamp: "t3".to_string(),
                item: RolloutItem::ResponseItem(user_message("keep-2")),
            },
            RolloutLine {
                timestamp: "t4".to_string(),
                item: RolloutItem::EventMsg(EventMsg::ContextCompacted(ContextCompactedEvent {})),
            },
            RolloutLine {
                timestamp: "t5".to_string(),
                item: RolloutItem::ResponseItem(user_message("after")),
            },
        ];

        let got = extract_between_last_two_context_compactions(&lines);
        let got_items: Vec<_> = got.into_iter().map(|l| l.item).collect();
        let expected = vec![
            RolloutItem::ResponseItem(user_message("keep-1")),
            RolloutItem::ResponseItem(user_message("keep-2")),
        ];
        assert_eq!(
            serde_json::to_value(got_items).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
    }

    #[test]
    fn strips_tool_output_telemetry_but_keeps_exit_code() {
        let raw = "Chunk ID: abc\nWall time: 1.2 seconds\nProcess exited with code 7\nOriginal token count: 123\nOutput:\nhello\nTotal output lines: 5\n";
        let cleaned = strip_tool_output_telemetry(raw);
        assert!(!cleaned.contains("Chunk ID:"));
        assert!(!cleaned.contains("Wall time:"));
        assert!(!cleaned.contains("Original token count:"));
        assert!(!cleaned.contains("Total output lines:"));
        assert!(cleaned.contains("Process exited with code 7"));
        assert!(cleaned.contains("hello"));
    }

    #[test]
    fn sanitize_for_seed_drops_reasoning_and_sanitizes_tool_output() {
        let tool_output = FunctionCallOutputPayload {
            content:
                "Chunk ID: a\nProcess exited with code 0\nWall time: 0.1 seconds\nOutput:\nok\n"
                    .to_string(),
            content_items: None,
            success: None,
        };
        let segment = vec![
            RolloutLine {
                timestamp: "t1".to_string(),
                item: RolloutItem::ResponseItem(ResponseItem::Reasoning {
                    id: "r".to_string(),
                    summary: Vec::new(),
                    content: None,
                    encrypted_content: None,
                }),
            },
            RolloutLine {
                timestamp: "t2".to_string(),
                item: RolloutItem::ResponseItem(ResponseItem::FunctionCallOutput {
                    call_id: "c1".to_string(),
                    output: tool_output,
                }),
            },
        ];

        let got = sanitize_for_seed(&segment);
        let got_items: Vec<_> = got.into_iter().map(|l| l.item).collect();

        let expected = vec![RolloutItem::ResponseItem(
            ResponseItem::FunctionCallOutput {
                call_id: "c1".to_string(),
                output: FunctionCallOutputPayload {
                    content: "Process exited with code 0\nOutput:\nok".to_string(),
                    content_items: None,
                    success: None,
                },
            },
        )];
        assert_eq!(
            serde_json::to_value(got_items).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
    }
}
