# Data Model and Parsing

All input is parsed eagerly into `FlatJson(rows, text, max_depth)`. Nothing is parsed lazily or re-read from the source; the model is immutable apart from collapse flags.

## Rows

A `Row` holds parent and sibling links (`OptionIndex`), depth, index within its parent, the byte range of its value in the canonical text, an optional key range (key including its delimiters), and a `Value`. Primitives and empty containers take one row. A non-empty container takes an `OpenContainer` row (first child, close index) and a `CloseContainer` row (last child, open index), both carrying `collapsed`; sibling links are set on the open row only. Multiple top-level values (newline-delimited JSON, multi-document YAML) are parentless siblings.

`FlatJson` provides the walkers every other layer uses: `prev/next_visible_row` skip the insides of collapsed containers, `prev/next_item` also skip closing rows (data mode), and `first_visible_ancestor` maps a hidden row to its outermost collapsed ancestor.

## Parsers

- **JSON** — a hand-written recursive-descent parser over a `logos` tokenizer. It accepts a sequence of top-level values separated by whitespace and fails fast with a string error.
- **YAML** — loads documents with `yaml-rust` and converts them to the same rows. Non-string scalar keys are rendered inside `[...]` instead of quotes, which path building recognizes.

Both emit the canonical text as they create rows, so ranges index into it directly.

## Canonical Text

Single-line JSON, one line per top-level value. It is the haystack for search and the source for rendered values, copied content, and paths. `jsonstringunescaper.rs` decodes string escapes (including surrogate pairs) for copying string contents and for sexp output.

## Serialization and Paths

- `pretty_printed` — the whole document, for non-TTY output and `:write`; `pretty_printed_value` — one subtree, for copy/print.
- `sexp_string` (feature `sexp`) — the whole document as S-expressions, for `:writesexp`.
- `build_path_to_node` — dot, bracket, and jq-style query paths for copy/print, plus a variant with top-level indices for the status bar.

## Code Entry Points

- `src/flatjson.rs` — `FlatJson`, `Row`, `Value`, walkers, serialization, paths, and the `parse_top_level_json`/`parse_top_level_yaml` entry points.
- `src/jsonparser.rs`, `src/jsontokenizer.rs` — JSON; `src/yamlparser.rs` — YAML.
- `src/jsonstringunescaper.rs` — safe (control characters kept escaped) and unsafe unescaping.
