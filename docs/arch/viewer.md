# Viewer State and Navigation

`JsonViewer` owns the `FlatJson`, the focused row, the top row of the window, the display mode, the scrolloff setting, and the viewport dimensions (excluding the status bar). It knows nothing about input or rendering.

## Modes

- **Line** — every visible row is a screen line, including closing delimiters; text is close to valid JSON.
- **Data** (default) — closing rows are hidden; keys are unquoted when they are JS identifiers; array indices and container previews are shown.

Every movement and line count switches on the mode to choose `*_visible_row` or `*_item` walkers ([data-model.md](data-model.md)). Toggling the mode moves focus off a closing row and keeps the focused line at the same screen position.

## Actions

All state changes go through `perform_action(Action)`: line and sibling movement, depth-change jumps, parent/first/last/matching-pair focus, scrolling, half-page jumps, paging, jump-to-line, `z` repositioning, clicks, collapse/expand (single, siblings, deep), mode toggle, and resize. Count prefixes arrive as action arguments.

After each action, one of three window policies applies, chosen per action:

- **Track focus** — keep the focused row visible with scrolloff padding; a large jump re-centers the focused row about a third of the way down.
- **Pin screen position** — keep the focused line at the same screen row (mode toggle, sibling collapse/expand).
- **Action-managed** — scrolling and jumps set the window themselves.

Sibling movement remembers a desired depth across consecutive presses so it can climb out and return to the original depth; most other actions reset it.

## Code Entry Points

- `src/viewer.rs` — `JsonViewer`, `Mode`, `Action`, `perform_action`, window tracking (`ensure_focused_row_is_visible`), and line counting helpers.
