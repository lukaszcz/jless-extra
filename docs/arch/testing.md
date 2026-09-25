# Testing

All tests are unit tests in a `#[cfg(test)] mod tests` at the bottom of the module they cover; there is no separate integration suite and no test drives a real terminal.

## Strategy

- **Parsers and data model** — parse literal JSON/YAML strings and assert row structure, canonical text, paths, and serialization.
- **Viewer** — build a `JsonViewer` from literal JSON, apply sequences of `Action`s, and assert focused and top rows in both modes.
- **Rendering** — run `LinePrinter` against `TextOnlyTerminal` (plain text) or `VisibleEscapesTerminal` (style changes as readable markers) and compare lines. `TruncatedStrView` and the unescaper have table-driven tests.
- **Search** — assert match ranges and jump destinations over a fixed document.

`App`, `ScreenWriter`, and `input` are not unit tested; changes there are verified by running the binary.

## Gates

`just check` is the single gate: rustfmt, clippy with `-D warnings` over all targets, and the tests, each with and without the `sexp` feature. CI (`.github/workflows/ci.yml`) runs it with stable Rust on Linux x86_64, Linux ARM64, and macOS (Apple Silicon).

## Code Entry Points

- `src/terminal.rs` — `terminal::test` with `TextOnlyTerminal` and `VisibleEscapesTerminal`.
- `src/viewer.rs`, `src/lineprinter.rs`, `src/flatjson.rs`, `src/search.rs`, `src/truncatedstrview.rs`, `src/jsonparser.rs`, `src/yamlparser.rs`, `src/jsonstringunescaper.rs` — test modules.
- `justfile` — `test`, `lint`, and `check` recipes; `.github/workflows/ci.yml` — runs `just check`; `.github/workflows/release.yml` — tagged release binaries.
