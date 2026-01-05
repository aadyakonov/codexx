use std::sync::Arc;

use crate::Prompt;
use crate::codex::Session;
use crate::codex::TurnContext;
use crate::error::CodexErr;
use crate::error::Result as CodexResult;
use crate::protocol::CompactedItem;
use crate::protocol::ContextCompactedEvent;
use crate::protocol::EventMsg;
use crate::protocol::RolloutItem;
use crate::protocol::TaskStartedEvent;
use codex_protocol::models::ResponseItem;
use codex_protocol::user_input::UserInput;

pub(crate) async fn run_inline_remote_auto_compact_task(
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
) {
    run_remote_compact_task_inner(&sess, &turn_context, None).await;
}

pub(crate) async fn run_remote_compact_task(
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
    compaction_instructions: Option<String>,
) {
    let start_event = EventMsg::TaskStarted(TaskStartedEvent {
        model_context_window: turn_context.client.get_model_context_window(),
    });
    sess.send_event(&turn_context, start_event).await;

    run_remote_compact_task_inner(&sess, &turn_context, compaction_instructions).await;
}

async fn run_remote_compact_task_inner(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
    compaction_instructions: Option<String>,
) {
    let compact_instructions =
        compaction_instructions.unwrap_or_else(|| turn_context.resolve_compact_prompt());

    if let Err(err) =
        run_remote_compact_task_inner_impl(sess, turn_context, &compact_instructions).await
    {
        if matches!(err, CodexErr::InvalidRequest(_)) {
            sess.notify_background_event(
                turn_context.as_ref(),
                format!("Remote compaction failed; falling back to local compaction: {err}"),
            )
            .await;
            crate::compact::run_compact_task_inner(
                Arc::clone(sess),
                Arc::clone(turn_context),
                vec![UserInput::Text {
                    text: compact_instructions,
                }],
            )
            .await;
        } else {
            let event = EventMsg::Error(
                err.to_error_event(Some("Error running remote compact task".to_string())),
            );
            sess.send_event(turn_context, event).await;
        }
    }
}

async fn run_remote_compact_task_inner_impl(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
    compact_instructions: &str,
) -> CodexResult<()> {
    let mut history = sess.clone_history().await;

    let prompt = Prompt {
        input: history.get_history_for_prompt(),
        tools: vec![],
        parallel_tool_calls: false,
        base_instructions_override: Some(compact_instructions.to_string()),
        output_schema: None,
    };

    let mut new_history = turn_context
        .client
        .compact_conversation_history(&prompt)
        .await?;
    // Required to keep `/undo` available after compaction
    let ghost_snapshots: Vec<ResponseItem> = history
        .get_history()
        .iter()
        .filter(|item| matches!(item, ResponseItem::GhostSnapshot { .. }))
        .cloned()
        .collect();

    if !ghost_snapshots.is_empty() {
        new_history.extend(ghost_snapshots);
    }
    sess.replace_history(new_history.clone()).await;
    sess.recompute_token_usage(turn_context).await;

    let compacted_item = CompactedItem {
        message: String::new(),
        replacement_history: Some(new_history),
    };
    sess.persist_rollout_items(&[RolloutItem::Compacted(compacted_item)])
        .await;

    let event = EventMsg::ContextCompacted(ContextCompactedEvent {});
    sess.send_event(turn_context, event).await;

    Ok(())
}
