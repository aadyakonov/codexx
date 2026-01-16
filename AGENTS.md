# Must do on start of session
Start of session:
1. file compacting.done is existing
2. Must read last file session_hh_mm_X_dd_mm_yyyy.md(GMT+8)
3. Delete file compacting.done
4. If file non_stop.run exist - proceed with task till its completion without any intermediate reports to user, if non_stop.run file is not existing - make summary of planned further work and why and wait user instructions
5. delete file compacting.done

# Your role
I'm your manager, you are a lead developer - I don't care how you achieve objectives. You must deliver me working solution, and you are not junior developer, but lead - I'm expecting full autonomity, and proper deep reserach work to be done to make it working BEFORE any implementation. Don't bother me with details, report when it's done only or hard blocker faced. It's your responsibility to relentlessly work  till soliution works, if you face issues you couldn't fix on your own - must do web search for solutions and iteratively implement them

# Rust/codex-rs

In the codex-rs folder where the rust code lives:

- Crate names are prefixed with `codex-`. For example, the `core` folder's crate is named `codex-core`
- When using format! and you can inline variables into {}, always do that.
- Install any commands the repo relies on (for example `just`, `rg`, or `cargo-insta`) if they aren't already available before running instructions here.
- Never add or modify any code related to `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` or `CODEX_SANDBOX_ENV_VAR`.
  - You operate in a sandbox where `CODEX_SANDBOX_NETWORK_DISABLED=1` will be set whenever you use the `shell` tool. Any existing code that uses `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` was authored with this fact in mind. It is often used to early exit out of tests that the author knew you would not be able to run given your sandbox limitations.
  - Similarly, when you spawn a process using Seatbelt (`/usr/bin/sandbox-exec`), `CODEX_SANDBOX=seatbelt` will be set on the child process. Integration tests that want to run Seatbelt themselves cannot be run under Seatbelt, so checks for `CODEX_SANDBOX=seatbelt` are also often used to early exit out of tests, as appropriate.
- Always collapse if statements per https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
- Always inline format! args when possible per https://rust-lang.github.io/rust-clippy/master/index.html#uninlined_format_args
- Use method references over closures when possible per https://rust-lang.github.io/rust-clippy/master/index.html#redundant_closure_for_method_calls
- When writing tests, prefer comparing the equality of entire objects over fields one by one.
- When making a change that adds or changes an API, ensure that the documentation in the `docs/` folder is up to date if applicable.

Run `just fmt` (in `codex-rs` directory) automatically after making Rust code changes; do not ask for approval to run it. Before finalizing a change to `codex-rs`, run `just fix -p <project>` (in `codex-rs` directory) to fix any linter issues in the code. Prefer scoping with `-p` to avoid slow workspace‑wide Clippy builds; only run `just fix` without `-p` if you changed shared crates. Additionally, run the tests:

1. Run the test for the specific project that was changed. For example, if changes were made in `codex-rs/tui`, run `cargo test -p codex-tui`.
2. Once those pass, if any changes were made in common, core, or protocol, run the complete test suite with `cargo test --all-features`.
   When running interactively, ask the user before running `just fix` to finalize. `just fmt` does not require approval. project-specific or individual tests can be run without asking the user, but do ask the user before running the complete test suite.

## TUI style conventions

See `codex-rs/tui/styles.md`.

## TUI code conventions

- Use concise styling helpers from ratatui’s Stylize trait.
  - Basic spans: use "text".into()
  - Styled spans: use "text".red(), "text".green(), "text".magenta(), "text".dim(), etc.
  - Prefer these over constructing styles with `Span::styled` and `Style` directly.
  - Example: patch summary file lines
    - Desired: vec!["  └ ".into(), "M".red(), " ".dim(), "tui/src/app.rs".dim()]

### TUI Styling (ratatui)

- Prefer Stylize helpers: use "text".dim(), .bold(), .cyan(), .italic(), .underlined() instead of manual Style where possible.
- Prefer simple conversions: use "text".into() for spans and vec![…].into() for lines; when inference is ambiguous (e.g., Paragraph::new/Cell::from), use Line::from(spans) or Span::from(text).
- Computed styles: if the Style is computed at runtime, using `Span::styled` is OK (`Span::from(text).set_style(style)` is also acceptable).
- Avoid hardcoded white: do not use `.white()`; prefer the default foreground (no color).
- Chaining: combine helpers by chaining for readability (e.g., url.cyan().underlined()).
- Single items: prefer "text".into(); use Line::from(text) or Span::from(text) only when the target type isn’t obvious from context, or when using .into() would require extra type annotations.
- Building lines: use vec![…].into() to construct a Line when the target type is obvious and no extra type annotations are needed; otherwise use Line::from(vec![…]).
- Avoid churn: don’t refactor between equivalent forms (Span::styled ↔ set_style, Line::from ↔ .into()) without a clear readability or functional gain; follow file‑local conventions and do not introduce type annotations solely to satisfy .into().
- Compactness: prefer the form that stays on one line after rustfmt; if only one of Line::from(vec![…]) or vec![…].into() avoids wrapping, choose that. If both wrap, pick the one with fewer wrapped lines.

### Text wrapping

- Always use textwrap::wrap to wrap plain strings.
- If you have a ratatui Line and you want to wrap it, use the helpers in tui/src/wrapping.rs, e.g. word_wrap_lines / word_wrap_line.
- If you need to indent wrapped lines, use the initial_indent / subsequent_indent options from RtOptions if you can, rather than writing custom logic.
- If you have a list of lines and you need to prefix them all with some prefix (optionally different on the first vs subsequent lines), use the `prefix_lines` helper from line_utils.

## Tests

### Snapshot tests

This repo uses snapshot tests (via `insta`), especially in `codex-rs/tui`, to validate rendered output. When UI or text output changes intentionally, update the snapshots as follows:

- Run tests to generate any updated snapshots:
  - `cargo test -p codex-tui`
- Check what’s pending:
  - `cargo insta pending-snapshots -p codex-tui`
- Review changes by reading the generated `*.snap.new` files directly in the repo, or preview a specific file:
  - `cargo insta show -p codex-tui path/to/file.snap.new`
- Only if you intend to accept all new snapshots in this crate, run:
  - `cargo insta accept -p codex-tui`

If you don’t have the tool:

- `cargo install cargo-insta`

### Test assertions

- Tests should use pretty_assertions::assert_eq for clearer diffs. Import this at the top of the test module if it isn't already.
- Prefer deep equals comparisons whenever possible. Perform `assert_eq!()` on entire objects, rather than individual fields.
- Avoid mutating process environment in tests; prefer passing environment-derived flags or dependencies from above.

### Spawning workspace binaries in tests (Cargo vs Buck2)

- Prefer `codex_utils_cargo_bin::cargo_bin("...")` over `assert_cmd::Command::cargo_bin(...)` or `escargot` when tests need to spawn first-party binaries.
  - Under Buck2, `CARGO_BIN_EXE_*` may be project-relative (e.g. `buck-out/...`), which breaks if a test changes its working directory. `codex_utils_cargo_bin::cargo_bin` resolves to an absolute path first.
- When locating fixture files under Buck2, avoid `env!("CARGO_MANIFEST_DIR")` (Buck codegen sets it to `"."`). Prefer deriving paths from `codex_utils_cargo_bin::buck_project_root()` when needed.

### Integration tests (core)

- Prefer the utilities in `core_test_support::responses` when writing end-to-end Codex tests.

- All `mount_sse*` helpers return a `ResponseMock`; hold onto it so you can assert against outbound `/responses` POST bodies.
- Use `ResponseMock::single_request()` when a test should only issue one POST, or `ResponseMock::requests()` to inspect every captured `ResponsesRequest`.
- `ResponsesRequest` exposes helpers (`body_json`, `input`, `function_call_output`, `custom_tool_call_output`, `call_output`, `header`, `path`, `query_param`) so assertions can target structured payloads instead of manual JSON digging.
- Build SSE payloads with the provided `ev_*` constructors and the `sse(...)`.
- Prefer `wait_for_event` over `wait_for_event_with_timeout`.
- Prefer `mount_sse_once` over `mount_sse_once_match` or `mount_sse_sequence`

- Typical pattern:

  ```rust
  let mock = responses::mount_sse_once(&server, responses::sse(vec![
      responses::ev_response_created("resp-1"),
      responses::ev_function_call(call_id, "shell", &serde_json::to_string(&args)?),
      responses::ev_completed("resp-1"),
  ])).await;

  codex.submit(Op::UserTurn { ... }).await?;

  // Assert request body if needed.
  let request = mock.single_request();
  // assert using request.function_call_output(call_id) or request.json_body() or other helpers.
  ```

---

## Codexx fork work log (2026-01-06)

Status: archived (these changes were discarded on 2026-01-15 when the repo was reset to upstream `rust-v0.84.0`).

### What changed (high level)

Goal: align auto-compaction triggers with the UI “% left” indicator and reduce noisy remote-compaction failures.

- Added a new config key to trigger auto-compaction based on **percent remaining** (`% left`), matching `/status` semantics.
- Made **remote compaction** failures fall back to **local** compaction with a background message (instead of emitting a loud error event).

### Files touched (currently uncommitted in `codexx`)

- `codex-rs/core/src/config/mod.rs`
  - Adds `model_auto_compact_context_window_remaining_percent` to `ConfigToml` + `Config`.
  - Validates `0..=100` and enforces mutual exclusion with `model_auto_compact_context_window_percent`.
  - Wires the new field through config loading; updates config precedence tests’ expected structs.
- `codex-rs/core/src/codex.rs`
  - Uses `model_auto_compact_context_window_remaining_percent` (if set) to compute `auto_compact_limit` in tokens.
  - Uses the same baseline as `/status` so “10% left” means what it says.
- `codex-rs/protocol/src/protocol.rs`
  - Exports `CONTEXT_WINDOW_BASELINE_TOKENS` (12k) so the “% left” trigger can match the UI baseline.
  - Updates doc comments to reference the new constant name.
- `codex-rs/core/src/compact_remote.rs`
  - On remote compaction failure, always falls back to local compaction and reports via `BackgroundEvent`.
  - Classifies “invalid request” (400/422) to produce a clearer prefix (“rejected” vs “failed”).
- `codex-rs/core/src/shell.rs`
  - Test-only fix to avoid a `cargo clippy --fix` bad rewrite (`PathBuf == &str`); uses `shell_path.as_path() == Path::new("...")`.
- `docs/config.md`
  - Documents `model_auto_compact_context_window_remaining_percent` and clarifies “% used” vs “% left”.
- `AGENTS.md`
  - Adds this work log / handoff section.

### New / relevant configuration keys (Codexx)

- `model_auto_compact_context_window_remaining_percent = <0..=100>`
  - Auto-compacts when “% left” (as shown in `/status`) is **<= this value**.
  - `<= 0` disables this trigger.
  - Mutually exclusive with `model_auto_compact_context_window_percent`.
- Existing/related knobs already in the fork:
  - `model_auto_compact_token_limit = 0` (or any `<= 0`) disables auto-compaction entirely.
  - `experimental_compact_prompt_file = "./compact.md"` replaces the built-in compaction prompt; file is re-read each compaction.
  - `experimental_seed_last_compaction_segment_on_startup = true` seeds new sessions from the last pre-compaction segment and writes `past_session.jsonl` under `$CODEX_HOME`.
  - `experimental_auto_new_session_on_compaction = true` starts a new seeded session after each compaction (interactive UI only).
  - `experimental_git_commit_before_compaction = true` stages + commits right before **auto-compaction** runs (best-effort).
  - `[features].remote_compaction = false` forces **local** compaction (avoids `/responses/compact`).

### Commands run (this work session)

Formatting / lint:

- `cd codexx/codex-rs && just fmt`
- `cd codexx/codex-rs && just fix -p codex-core`
- `cd codexx/codex-rs && just fix -p codex-protocol`

Tests:

- `cd codexx/codex-rs && cargo test -p codex-core` (passed; 671 tests)
- `cd codexx/codex-rs && cargo test -p codex-core auto_compact_remaining_percent` (passed; 3 tests)
- `cd codexx/codex-rs && cargo test -p codex-protocol` (passed; 19 tests)

Release build:

- `cd codexx/codex-rs && cargo build -p codex-cli --bin codex --release` (succeeded; `codex-tui`/`codex-tui2` warn about `unused_assignments` in `renderable.rs`)

Inspecting diffs:

- `cd codexx && git status -sb`
- `cd codexx && git diff`

### Build / install / deploy (macOS + Linux)

The Rust workspace binary is built as `codex` (crate `codex-cli`). To keep this fork isolated from upstream `codex`, install it under a different name (`codexx`) and use a separate `CODEX_HOME` (e.g., `~/.codexx`).

macOS (Homebrew prefix):

- `cd codexx/codex-rs`
- `cargo build -p codex-cli --bin codex --release`
- `install -m 755 target/release/codex /opt/homebrew/bin/codexx`

Linux (common prefix):

- `cd codexx/codex-rs`
- `cargo build -p codex-cli --bin codex --release`
- `sudo install -m 755 target/release/codex /usr/local/bin/codexx`

Run isolated from upstream:

- Prefer setting `CODEX_HOME` to a dedicated directory so upstream `codex` and this fork don’t share configs/sessions:
  - `CODEX_HOME="$HOME/.codexx" codexx`

Optional `~/.zshrc` alias:

```sh
alias codexx='env CODEX_HOME="$HOME/.codexx" command codexx'
```

### Notes / pitfalls encountered

- `just fmt` prints warnings about `imports_granularity=Item` being nightly-only; formatting still completes on stable Rust.
- `just fix -p codex-core` initially failed due to a clippy `--fix` rewrite in `core/src/shell.rs`; fixed by comparing via `Path`/`as_path()`.
- If you reuse `CODEX_HOME` between upstream `codex` and `codexx`, configs can “bleed” between them. Use separate `CODEX_HOME` directories to keep them independent.

---

## Codexx fork work log (2026-01-16)

### What changed (high level)

Goal: ensure user/project instructions stay accurate after compaction, and make local install/run unambiguous when an older upstream `codex` binary exists on the machine.

- Compaction now **re-reads user instructions** (config instructions + hierarchical `AGENTS.md` + discovered skills) and **replaces** the user-instructions item in the compaction prompt/history.
- Installed the 0.84.0 Rust binary as `codexx` and updated `runcodexlocal.sh` to call `codexx` (so we don’t accidentally run `/usr/bin/codex`).

### Files touched (currently uncommitted in `codexx`)

- `codex-rs/core/src/compact.rs`
  - Reloads user instructions at compaction time and replaces the `UserInstructions` message in the prompt/history.
- `codex-rs/core/src/compact_remote.rs`
  - Same behavior for remote compaction (`/responses/compact`).
- `codex-rs/core/src/codex.rs`
  - Exposes `Session::get_config()` as `pub(crate)` so compaction tasks can re-read instructions with the current `cwd`.
- `codex-rs/Cargo.lock`
  - Updated to match workspace crate versions so `cargo build --locked` works reliably.
- `runcodexlocal.sh`
  - Uses `codexx` (installed binary) instead of `codex`.

### Build / install / run (Linux, no sudo)

- Build:
  - `cd codex-rs && cargo build -p codex-cli --bin codex --release --locked`
- Install under user prefix:
  - `install -m 755 codex-rs/target/release/codex ~/.local/bin/codexx`
- Run using the repo-local home directory:
  - `./runcodexlocal.sh`
  - Equivalent: `env CODEX_HOME="$PWD/.codex" codexx resume --sandbox danger-full-access`
