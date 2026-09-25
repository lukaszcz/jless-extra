# Search

Search runs a regex over the canonical text ([data-model.md](data-model.md)) once, when the search starts, and stores every match as a byte range. Navigation maps a match back to the row containing it.

## Semantics

- Smart case: case-insensitive unless the term has an uppercase letter; a `/s` suffix forces case sensitivity, a trailing `/` is stripped.
- Square and curly bracket escaping is inverted, so `[` and `{` match literally and `\[` / `\{` are regex syntax.
- `*` / `#` search for the focused object key followed by `: `.
- Because the haystack is the canonical text, matches can span keys, quotes, and punctuation as they appear there, not as displayed.

## Navigation and State

`jump_to_match` finds the next or previous match relative to the focused row, honoring the original search direction and counts, wraps around the document, and returns the destination row. A match inside a collapsed container focuses its first visible ancestor; the next jump continues from the matches inside it rather than skipping past it.

`SearchState` tracks whether matches are hidden, visible, or actively being navigated (with the current match and a wrap flag); `App` updates this after each event ([app.md](app.md)), and the renderer highlights matches and the current match accordingly ([rendering.md](rendering.md)).

## Code Entry Points

- `src/search.rs` — `SearchState`, term parsing, match computation, `jump_to_match`, match iteration for rendering.
- `src/app.rs` — prompts, `*`/`#`, and active-search bookkeeping.
