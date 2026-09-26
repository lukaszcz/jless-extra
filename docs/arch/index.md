# jless Architecture Overview

jless is a terminal pager for JSON and YAML. It parses the whole input up front into a flat array of rows, then runs a single-threaded event loop that turns key, mouse, and resize events into viewer actions and redraws the screen after each one. When stdout is not a TTY it only pretty-prints the input and exits.

Start here for the system shape, then read only the subsystem documents relevant to the task.

## System Shape

- **Entry** — parses CLI options, reads the input file or stdin, picks the format (flag, else file extension, else JSON), and either pretty-prints (non-TTY stdout) or resolves the terminal theme, sets up the terminal, and starts the app.
- **Data model** — `FlatJson`: the row array, a canonical single-line text that rows index into, and the maximum depth. Built by the JSON or YAML parser; every other layer reads it.
- **Controller** — `App`: the event loop, multi-key input states and count prefixes, `:` commands, search, copy/print/write, help, and suspend.
- **Viewer** — `JsonViewer`: focused row, window position, display mode, and collapse state, changed only through `Action`s.
- **Rendering** — `ScreenWriter` draws the screen and status bar, delegating each row to `LinePrinter`, which uses truncation and highlighting helpers and writes to a buffered `Terminal`.
- **Search** — `SearchState`: regex matches over the canonical text and navigation between them.

## Architecture and Design Decisions

- **The tree is a flat array.** A non-empty container is two rows (open and close) that point at each other; parent, sibling, and child links are indices. Navigation is index walking, and collapsing sets a flag on both rows of a pair.
- **One canonical text backs everything.** Parsers re-emit the input as single-line JSON while building rows, and rows store byte ranges into it. Rendering, copying, paths, and search all slice that string; the original input text and formatting are discarded.
- **Modes are views, not data.** Line mode shows every visible row; data mode hides closing rows and quotes/commas. Both walk the same `FlatJson` with different step functions.
- **App interprets; viewer transitions.** `App` maps events to `Action`s; `JsonViewer::perform_action` applies them and then restores window invariants. Display-only state (indentation reduction, per-row horizontal scroll, line-number toggles) lives in `ScreenWriter`, not the viewer.
- **Full redraw per event.** Each event renders the whole screen into a string buffer and flushes it once; there is no diffing.
- **Output goes through a `Terminal` trait**, so rendering is testable without a TTY.
- **Optional functionality is a Cargo feature.** `sexp` gates S-expression output.

## What To Read Next

- [data-model.md](data-model.md) — rows, parsers, the canonical text, paths, and serialization.
- [viewer.md](viewer.md) — display modes, actions, focus movement, and window tracking.
- [rendering.md](rendering.md) — screen and status bar, line layout, truncation and horizontal scrolling, highlighting, the terminal abstraction.
- [app.md](app.md) — terminal setup, the event source, the input state machine, commands, clipboard, print, and suspend.
- [search.md](search.md) — search semantics and match navigation.
- [testing.md](testing.md) — test organization and CI gates.

## Code Entry Points

- `src/main.rs` — entry point, input reading, format detection, non-TTY pretty-printing, terminal setup; `src/options.rs` — clap CLI definition.
- `src/flatjson.rs` — the data model; `src/jsonparser.rs`, `src/jsontokenizer.rs`, `src/yamlparser.rs` — parsers; `src/jsonstringunescaper.rs` — JSON string unescaping.
- `src/app.rs` — event loop and key bindings; `src/input.rs` — event source.
- `src/viewer.rs` — viewer state and actions.
- `src/screenwriter.rs`, `src/lineprinter.rs`, `src/truncatedstrview.rs`, `src/highlighting.rs`, `src/terminal.rs` — rendering.
- `src/search.rs` — search.
- `src/jless.help` — in-app help text, embedded at compile time.
- `src/render-notes.md` and `SEARCH.md` are early design notes, not current documentation.
