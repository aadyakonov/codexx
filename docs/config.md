# Configuration

For basic configuration instructions, see [this documentation](https://developers.openai.com/codex/config-basic).

For advanced configuration instructions, see [this documentation](https://developers.openai.com/codex/config-advanced).

For a full configuration reference, see [this documentation](https://developers.openai.com/codex/config-reference).

## Codexx-specific experimental options

This fork adds a few additional `config.toml` keys:

```toml
# Use a custom prompt file for compaction (re-read each compaction).
experimental_compact_prompt_file = "./compact.md"

# Auto-compact at % of context window (<= 0 disables).
model_auto_compact_context_window_percent = 50

# Seed new sessions from the last pre-compaction segment and optionally auto-start a new one after compaction.
experimental_seed_last_compaction_segment_on_startup = true
experimental_auto_new_session_on_compaction = true

# Create a real git commit (git add -A + git commit) right before auto-compaction runs.
experimental_git_commit_before_compaction = true
```

### Notes

- `experimental_compact_prompt_file` **replaces** Codex’s built-in compaction prompt (it is not appended).
  - It is re-read on every compaction so you can edit `compact.md` mid-session.
  - If the file is empty or can’t be read, Codexx falls back to the cached/built-in prompt.
- `model_auto_compact_context_window_percent` is **percent used**, not percent remaining.
  - Example: “compact when **10% left**” ⇒ set `model_auto_compact_context_window_percent = 90`.
  - To fully disable auto-compaction, set `model_auto_compact_token_limit = 0` (or any `<= 0` value).
- Relative paths like `./compact.md` are resolved relative to the folder containing the `config.toml` that defined them (e.g. `~/.codexx/config.toml` or `<repo>/.codexx/config.toml`).
- To keep upstream `codex` and this fork isolated, use `CODEXX_HOME` (preferred) instead of `CODEX_HOME`.
  - Upstream `codex` reads `CODEX_HOME`.
  - This fork (`codexx`) prefers `CODEXX_HOME` and falls back to `CODEX_HOME` only if `CODEXX_HOME` is unset.
- If you want to force **local** compaction (and avoid the OpenAI `/responses/compact` endpoint), disable the feature flag:

```toml
[features]
remote_compaction = false
```

## Past-session seeding (pre-compaction “line of development”)

When enabled, Codexx can start a new session with a cleaned slice of a prior session’s history:

- Seed source: the segment between the last two `"Context compacted"` markers in the most recent rollout under `$CODEXX_HOME/sessions/`.
- Cleaning: drops telemetry/noise (token counts, wall time, etc) but preserves meaningful tool outcomes like `Process exited with code ...`.
- Output: writes the cleaned slice to `$CODEXX_HOME/past_session.jsonl` for inspection/reuse.

Related options:

```toml
experimental_seed_last_compaction_segment_on_startup = true
experimental_auto_new_session_on_compaction = true
```

## Auto git commit before auto-compaction

If enabled, Codexx will `git add -A` + `git commit` right before **auto-compaction** runs (best-effort; failures won’t block compaction):

```toml
experimental_git_commit_before_compaction = true
```

## “Base system instructions”

Codexx includes a built-in “base system prompt” for each model family (embedded in the binary). For GPT‑5.2 Codex, see:

- `codex-rs/core/models.json` (`base_instructions` field)
- `codex-rs/core/gpt_5_2_prompt.md` (the prompt content referenced by `models.json`)

## Connecting to MCP servers

Codexx (this fork) can connect to MCP servers configured in `~/.codexx/config.toml` (or `$CODEXX_HOME/config.toml`). See the configuration reference for the latest MCP server options:

- https://developers.openai.com/codex/config-reference

## Notify

Codex can run a notification hook when the agent finishes a turn. See the configuration reference for the latest notification settings:

- https://developers.openai.com/codex/config-reference
