use std::sync::Arc;

use super::SessionTask;
use super::SessionTaskContext;
use crate::codex::TurnContext;
use crate::state::TaskKind;
use async_trait::async_trait;
use codex_protocol::user_input::UserInput;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Default)]
pub(crate) struct CompactTask;

#[async_trait]
impl SessionTask for CompactTask {
    fn kind(&self) -> TaskKind {
        TaskKind::Compact
    }

    async fn run(
        self: Arc<Self>,
        session: Arc<SessionTaskContext>,
        ctx: Arc<TurnContext>,
        input: Vec<UserInput>,
        _cancellation_token: CancellationToken,
    ) -> Option<String> {
        let session = session.clone_session();
        if crate::compact::should_use_remote_compact_task(
            session.as_ref(),
            &ctx.client.get_provider(),
        ) {
            let compaction_instructions = input.iter().find_map(|item| match item {
                UserInput::Text { text } => Some(text.clone()),
                UserInput::Image { .. } => None,
                _ => None,
            });
            crate::compact_remote::run_remote_compact_task(session, ctx, compaction_instructions)
                .await
        } else {
            crate::compact::run_compact_task(session, ctx, input).await
        }

        None
    }
}
