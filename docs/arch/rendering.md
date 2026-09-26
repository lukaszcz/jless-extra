# Rendering

Rendering is a pure function of viewer state, search state, and a little display state kept in `ScreenWriter`. Each redraw writes the whole screen into a string buffer, then flushes it to stdout.

## Screen and Status Bar

`ScreenWriter` walks visible rows from the viewer's top row, builds a `LinePrinter` for each, and threads the search-match iterator through them. Each row prints through a `ClippedTerminal` that places its `\n`-separated lines on screen rows, dropping lines scrolled above the window or past its bottom. The two-line status bar shows the focused node's path and the file name, then a message, the active search with match counter and wrap marker, or a `:` placeholder, plus the pending count/command buffer. `ScreenWriter` also owns the `rustyline` editor used for `/`, `?`, and `:` prompts at the bottom line.

Display state here, not in the viewer: a per-row cache of truncated value views that preserves horizontal scroll position (`.`, `,`, `;`, and scroll-to-search-match) when wrapping is off.

## Line Layout

`LinePrinter` fills a line left to right from the available width: line number, focus/container indicator and indentation, label (key or array index), then value or container preview. Mode decides quoting, trailing commas, delimiters, and when previews appear. `LinePrinter::for_row` derives these from the viewer's `LineLayout`; `print_line` returns the number of lines printed.

With wrapping on, a primitive value that does not fit continues on further lines, indented to the value (or to the label when the value starts past mid-screen), with `↩` in the last column of each wrapped line. Everything else, and everything when wrapping is off or continuation lines would be too narrow, is truncated with ellipses, or replaced by `>` when nothing fits.

`TruncatedStrView` computes which slice of a string fits in a width (Unicode width and grapheme aware) and supports scrolling that window left, right, to an end, or to a range. `highlighting` applies syntax colors and search-match styles to a truncated slice, mapping canonical-text match ranges onto displayed text, which differs from the source (unquoted keys, synthesized indices).

## Terminal Abstraction

`Terminal` is a `fmt::Write` trait with cursor positioning, clearing, and style operations. `AnsiTerminal` emits escape sequences into a string buffer, maps semantic styles and colors through the selected light or dark theme, and tracks the current style to skip redundant codes. `NullTerminal` discards output, for measuring. Test terminals render plain text or visible escape names ([testing.md](testing.md)).

## Code Entry Points

- `src/screenwriter.rs` — `ScreenWriter`: screen, status bar, prompts, horizontal scroll; `ClippedTerminal`.
- `src/lineprinter.rs` — `LinePrinter`, `LineLayout`, wrapping (`wrap_ranges`, `line_of_offset`), and the line-mode vs data-mode rules (documented at the top of the file); `JS_IDENTIFIER`.
- `src/truncatedstrview.rs` — string fitting and scrolling.
- `src/highlighting.rs` — styles and match highlighting.
- `src/terminal.rs` — `Terminal`, `AnsiTerminal`, colors, `Style`, and the test terminals.
