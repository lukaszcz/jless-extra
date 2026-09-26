jless is a terminal pager for JSON and YAML data, written in Rust and shipped as a single binary.

## Tech stack

- Rust, edition 2024, latest stable toolchain (no minimum supported version is maintained; CI runs `just check` on stable, Linux x86_64/ARM64 and macOS)
- `termion` (terminal I/O), `rustyline` (prompts), `logos` (JSON tokenizer), `yaml-rust`, `regex`, `clap` 4 (derive), `base64` (OSC 52 clipboard encoding)

## Architecture and project structure

Read @docs/arch/index.md to understand jless implementation architecture.

**IMPORTANT**: Update docs/arch/**/*.md whenever jless implementation architecture changes – always keep these files up-to-date with the codebase.

The primary purpose of architecture docs in docs/arch/**/*.md is to provide agents with a quick but comprehensive overview of the system's architecture and the codebase. Treat the docs as an onboarding guide. When updating, do not add brittle implementation details, but do include info on where to find relevant codebase references. Be *radically* concise and precise - convey essential *high-level* architectural information without extra prose. Remove redundant prose, implementation details, and duplicated information. Architecture docs provide architectural overview *only* - implementation mechanisms should be explained in comments instead. Match the existing writing style and succinctness level.

## Build, Test, and Development Commands

Use `just` for the standard workflow:

- `just build` / `just release` build debug / optimized binaries
- `just run <file>` runs the viewer (needs a real TTY). With stdout redirected jless only pretty-prints, which is handy for non-interactive checks: `echo '{"a":[1,2]}' | cargo run -q | cat`
- `just test` runs `cargo test` with and without the `sexp` feature; `just test <name>` runs matching tests, e.g. `just test viewer::tests::test_jump`
- `just lint` runs `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings` with and without `sexp` (run `just fmt` to fix formatting)
- `just check` runs linting and tests together — the final gate: it MUST pass before every commit
- `just install` builds a release binary from this checkout and copies it to `~/.local/bin`

## Coding Style & Naming Conventions

- Formatting: default `rustfmt`.
- Crate-wide clippy allows live at the top of `src/main.rs`. Do NOT add `#[allow(...)]` attributes. If ignoring a lint is necessary, ALWAYS ask the user for permission and explain why.
- Gate code used only for S-expression support behind `#[cfg(feature = "sexp")]`.
- Be radically concise and precise in comments - convey essential information without extra prose.

## Testing Guidelines

- Unit tests live in a `#[cfg(test)] mod tests` at the bottom of the module they test. Rendering is tested through the test terminals in `terminal.rs` (`TextOnlyTerminal`, `VisibleEscapesTerminal`), never a real TTY.
- **IMPORTANT**: Every new feature should include tests that verify its correctness.
- **IMPORTANT**: Follow Test Driven Development (TDD). Write failing tests first, implement changes later to make the tests pass.
- **IMPORTANT**: For every bug found, add a regression test that fails because of the bug, then fix the bug and ensure the test passes.
- Avoid brittle tests. Test behavior, not implementation details. Do NOT test exact help, warning, or error messages.
- Do NOT add heavy validation or defensive assertions (defense-in-depth) to the code. Write appropriate tests instead. Only trivial `debug_assert!` preconditions are allowed.
- Make sure tests are not flaky; keep individual tests cheap.

## Commit Guidelines

- Commit format: a capitalized imperative sentence ending with a period (e.g., `Add :write <file> (and :write!) commands for writing input to a file.`).
- Keep commits focused; avoid mixing unrelated changes.
- Add user-visible changes to the `main` section at the top of `CHANGELOG.md` in the same commit.

## Documentation

- Keep `src/jless.help` (the in-app help shown by F1 / `:help`), `CHANGELOG.md`, and `--help` text (doc comments in `src/options.rs`) up to date with implemented functionality. `README.md` is a brief description and should not contain overwhelming details. The user guide lives on the separate `website` branch.
- ALWAYS keep comments up-to-date with the codebase.
- Avoid references to plans, milestones, or unversioned files in the docs and comments.

## Instructions

- NEVER duplicate code. Abstract common logic into parameterized functions and separate modules.
- Do NOT create new worktrees - edit the current worktree directly.
- Do NOT try to circumvent static analysis tools. Adapt the code to pass `just check` properly - do not ignore checks or suppress lints. If you absolutely need to bypass a static analysis tool, ALWAYS ask the user for approval and explain why this is necessary.
- Be concise and precise in your responses, comments, docs, and explanations.
- **IMPORTANT**: When finished, verify with `just check` and commit. Work is not done until `just check` passes.
