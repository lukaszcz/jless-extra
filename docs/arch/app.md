# Event Loop and Input

`App` owns the viewer, screen writer, search state, input state, count buffer, and status message. It is the only place that knows key bindings.

## Terminal Setup and Event Source

Input data may come from stdin, so before the app starts `/dev/tty` is reopened as stdin; both key reading and `rustyline` prompts then read the terminal. Stdout is wrapped for the alternate screen, hidden cursor, mouse tracking, and raw mode. The selected theme is resolved at startup; automatic selection queries the terminal background and preserves any user input read with the response. The event source reads stdin bytes and a `SIGWINCH` self-pipe, and parses them into `TuiEvent`s (key, mouse, resize, unknown bytes).

## Event Loop

Each event is handled, then the screen is redrawn and the message cleared. Handling is a small state machine:

- **Default** — digits accumulate a count prefix; most keys map directly to an `Action` for the viewer ([viewer.md](viewer.md)); some call `ScreenWriter` for display-only changes.
- **Pending `p` / `y` / `z`** — the next key selects a print target, copy target, or window reposition; any other key cancels.
- **Waiting for any key** — after printing content to the main screen, the next key returns to the viewer.

`App` also decides whether a search stays "active": jumping to a match keeps it, moving focus ends it, and toggling collapse on the focused row keeps matches visible without a current match ([search.md](search.md)).

## Side-Effecting Commands

- **`:` commands** — parsed into a `Command`: quit, help, `set [no]number`/`[no]relativenumber`/`...!`, and `write`/`writesexp` (with `!` to overwrite).
- **Copy and print** — one content extractor (pretty or one-line value, unescaped string, key, dot/bracket/query path) feeds OSC 52 clipboard writes and printing to the main screen, where mouse button tracking is disabled so the user can select text.
- **Help** — pipes the embedded `jless.help` to `less -r` on the main screen.
- **Suspend** — Ctrl-Z restores the terminal, sends `SIGSTOP`, then restores the TUI and redraws.

## Code Entry Points

- `src/app.rs` — `App`, `InputState`, key bindings in `run`, `parse_command`, content extraction, copy/print/write, help, suspend.
- `src/osc52.rs` — base64 encoding and OSC 52 clipboard output.
- `src/input.rs` — `/dev/tty` remapping, `TuiEvent`, the poll-based event iterator.
- `src/main.rs` — terminal wrapping and app construction.
