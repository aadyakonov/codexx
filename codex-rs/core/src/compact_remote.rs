use std::sync::Arc;

use crate::Prompt;
use crate::codex::Session;
use crate::codex::TurnContext;
use crate::error::Result as CodexResult;
use crate::project_doc::get_user_instructions;
use crate::protocol::CompactedItem;
use crate::protocol::ContextCompactedEvent;
use crate::protocol::EventMsg;
use crate::protocol::RolloutItem;
use crate::protocol::TurnStartedEvent;
use crate::user_instructions::UserInstructions;
use codex_protocol::models::ResponseItem;

pub(crate) async fn run_inline_remote_auto_compact_task(
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
) {
    run_remote_compact_task_inner(&sess, &turn_context).await;
}

pub(crate) async fn run_remote_compact_task(sess: Arc<Session>, turn_context: Arc<TurnContext>) {
    let start_event = EventMsg::TurnStarted(TurnStartedEvent {
        model_context_window: turn_context.client.get_model_context_window(),
    });
    sess.send_event(&turn_context, start_event).await;

    run_remote_compact_task_inner(&sess, &turn_context).await;
}

fn is_developer_message(item: &ResponseItem) -> bool {
    matches!(item, ResponseItem::Message { role, .. } if role == "developer")
}

fn is_user_instructions_message(item: &ResponseItem) -> bool {
    let ResponseItem::Message { role, content, .. } = item else {
        return false;
    };
    if role != "user" {
        return false;
    }
    UserInstructions::is_user_instructions(content)
}

fn replace_user_instructions_items(
    mut items: Vec<ResponseItem>,
    replacement: Option<ResponseItem>,
) -> Vec<ResponseItem> {
    let Some(replacement) = replacement else {
        return items;
    };

    let prefix_len = items
        .iter()
        .take_while(|item| is_developer_message(item))
        .count();

    let mut out: Vec<ResponseItem> = Vec::with_capacity(items.len() + 1);
    out.extend(items.drain(..prefix_len));
    out.push(replacement);
    out.extend(
        items
            .into_iter()
            .filter(|item| !is_user_instructions_message(item)),
    );
    out
}

async fn run_remote_compact_task_inner(sess: &Arc<Session>, turn_context: &Arc<TurnContext>) {
    if let Err(err) = run_remote_compact_task_inner_impl(sess, turn_context).await {
        let event = EventMsg::Error(
            err.to_error_event(Some("Error running remote compact task".to_string())),
        );
        sess.send_event(turn_context, event).await;
    }
}

async fn run_remote_compact_task_inner_impl(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
) -> CodexResult<()> {
    let refreshed_user_instructions = {
        let mut cfg = (*sess.get_config().await).clone();
        cfg.cwd.clone_from(&turn_context.cwd);
        let skills_outcome = sess
            .services
            .skills_manager
            .skills_for_cwd(&turn_context.cwd, false)
            .await;
        get_user_instructions(&cfg, Some(&skills_outcome.skills)).await
    };
    let effective_user_instructions =
        refreshed_user_instructions.or_else(|| turn_context.user_instructions.clone());
    let user_instructions_item: Option<ResponseItem> =
        effective_user_instructions.as_ref().map(|text| {
            UserInstructions {
                directory: turn_context.cwd.to_string_lossy().into_owned(),
                text: text.clone(),
            }
            .into()
        });

    let history = sess.clone_history().await;

    // Required to keep `/undo` available after compaction
    let ghost_snapshots: Vec<ResponseItem> = history
        .raw_items()
        .iter()
        .filter(|item| matches!(item, ResponseItem::GhostSnapshot { .. }))
        .cloned()
        .collect();

    let prompt = Prompt {
        input: replace_user_instructions_items(
            history.for_prompt(),
            user_instructions_item.clone(),
        ),
        tools: vec![],
        parallel_tool_calls: false,
        base_instructions_override: turn_context.base_instructions.clone(),
        output_schema: None,
    };

    let mut new_history = turn_context
        .client
        .compact_conversation_history(&prompt)
        .await?;
    new_history = replace_user_instructions_items(new_history, user_instructions_item);

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
