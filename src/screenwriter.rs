use std::collections::HashMap;
use std::fmt::Write;
use std::io::Write as _;
use std::iter::Peekable;
use std::ops::Range;

use rustyline::Editor;
use termion::cursor::HideCursor;
use termion::input::MouseTerminal;
use termion::raw::RawTerminal;
use termion::screen::AlternateScreen;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::app::MAX_BUFFER_SIZE;
use crate::flatjson::{Index, OptionIndex, PathType, Row, Value};
use crate::lineprinter::LinePrinter;
use crate::search::{MatchRangeIter, SearchState};
use crate::terminal;
use crate::terminal::{AnsiTerminal, Color, Style, Terminal, Theme};
use crate::truncatedstrview::{TruncatedStrSlice, TruncatedStrView};
use crate::types::TTYDimensions;
use crate::viewer::{JsonViewer, Mode};

/// Raw-mode stdout on the alternate screen, with a hidden cursor and mouse tracking.
pub type TtyOutput = MouseTerminal<HideCursor<AlternateScreen<RawTerminal<std::io::Stdout>>>>;

pub struct ScreenWriter {
    pub stdout: TtyOutput,
    pub command_editor: Editor<()>,
    pub dimensions: TTYDimensions,
    pub terminal: AnsiTerminal,

    truncated_row_value_views: HashMap<Index, TruncatedStrView>,
}

pub enum MessageSeverity {
    Info,
    Warn,
    Error,
}

impl MessageSeverity {
    pub fn color(&self) -> terminal::Color {
        match self {
            MessageSeverity::Info => terminal::WHITE,
            MessageSeverity::Warn => terminal::YELLOW,
            MessageSeverity::Error => terminal::RED,
        }
    }
}

const PATH_BASE: &str = "input";
const SPACE_BETWEEN_PATH_AND_FILENAME: isize = 3;

impl ScreenWriter {
    pub fn init(
        stdout: TtyOutput,
        command_editor: Editor<()>,
        dimensions: TTYDimensions,
        theme: Theme,
    ) -> Self {
        let mut terminal = AnsiTerminal::new(String::new());
        terminal.set_theme(theme);

        ScreenWriter {
            stdout,
            command_editor,
            dimensions,
            terminal,
            truncated_row_value_views: HashMap::new(),
        }
    }

    pub fn print(
        &mut self,
        viewer: &JsonViewer,
        input_buffer: &[u8],
        input_filename: &str,
        search_state: &SearchState,
        message: &Option<(String, MessageSeverity)>,
    ) {
        self.print_viewer(viewer, search_state);
        self.print_status_bar(viewer, input_buffer, input_filename, search_state, message);
    }

    pub fn print_viewer(&mut self, viewer: &JsonViewer, search_state: &SearchState) {
        match self.print_screen_impl(viewer, search_state) {
            Ok(_) => match self.terminal.flush_contents(&mut self.stdout) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error while printing viewer: {e}");
                }
            },
            Err(e) => {
                eprintln!("Error while printing viewer: {e}");
            }
        }
    }

    pub fn print_status_bar(
        &mut self,
        viewer: &JsonViewer,
        input_buffer: &[u8],
        input_filename: &str,
        search_state: &SearchState,
        message: &Option<(String, MessageSeverity)>,
    ) {
        match self.print_status_bar_impl(
            viewer,
            input_buffer,
            input_filename,
            search_state,
            message,
        ) {
            Ok(_) => match self.terminal.flush_contents(&mut self.stdout) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error while printing status bar: {e}");
                }
            },
            Err(e) => {
                eprintln!("Error while printing status bar: {e}");
            }
        }
    }

    fn print_screen_impl(
        &mut self,
        viewer: &JsonViewer,
        search_state: &SearchState,
    ) -> std::fmt::Result {
        let mut line = OptionIndex::Index(viewer.top_row);
        let mut lines_to_skip = viewer.top_row_offset;
        let mut search_matches = search_state
            .matches_iter(viewer.flatjson[line.unwrap()].range.start)
            .peekable();
        let current_match = search_state.current_match_range();

        let mut delta_to_focused_row = viewer.rows_before_focused_row() as isize;

        let mut screen_index = 0;
        while screen_index < viewer.dimensions.height {
            match line {
                OptionIndex::Nil => {
                    self.terminal.position_cursor(1, screen_index + 1)?;
                    self.terminal.clear_line()?;
                    self.terminal.set_fg(terminal::LIGHT_BLACK)?;
                    self.terminal.write_char('~')?;
                    screen_index += 1;
                }
                OptionIndex::Index(index) => {
                    let max_lines = (viewer.dimensions.height - screen_index) as usize;
                    let mut terminal = ClippedTerminal {
                        inner: &mut self.terminal,
                        line: 0,
                        lines_to_skip,
                        first_screen_row: screen_index + 1,
                        max_lines,
                    };
                    terminal.start()?;

                    let row_lines = Self::print_line(
                        &mut terminal,
                        &mut self.truncated_row_value_views,
                        viewer,
                        index,
                        delta_to_focused_row,
                        &mut search_matches,
                        &current_match,
                    )?;

                    screen_index += (row_lines - lines_to_skip).min(max_lines) as u16;
                    lines_to_skip = 0;
                    line = match viewer.mode {
                        Mode::Line => viewer.flatjson.next_visible_row(index),
                        Mode::Data => viewer.flatjson.next_item(index),
                    };
                }
            }

            delta_to_focused_row -= 1;
        }

        Ok(())
    }

    pub fn get_command(&mut self, prompt: &str) -> rustyline::Result<String> {
        write!(self.stdout, "{}", termion::cursor::Show)?;
        let _ = self.terminal.position_cursor(1, self.dimensions.height);
        self.terminal.flush_contents(&mut self.stdout)?;

        let result = self.command_editor.readline(prompt);
        write!(self.stdout, "{}", termion::cursor::Hide)?;

        let _ = self.terminal.position_cursor(1, self.dimensions.height);
        let _ = self.terminal.clear_line();
        self.terminal.flush_contents(&mut self.stdout)?;

        result
    }

    // Prints a row and returns the number of screen lines it takes.
    fn print_line(
        terminal: &mut dyn Terminal,
        truncated_row_value_views: &mut HashMap<Index, TruncatedStrView>,
        viewer: &JsonViewer,
        index: Index,
        delta_to_focused_row: isize,
        search_matches: &mut Peekable<MatchRangeIter>,
        focused_search_match: &Range<usize>,
    ) -> Result<usize, std::fmt::Error> {
        let row = &viewer.flatjson[index];
        let focused = index == viewer.focused_row;
        let focused_because_matching_container_pair =
            row.is_container() && (focused || viewer.focused_row == row.pair_index().unwrap());

        let mut line = LinePrinter::for_row(
            terminal,
            &viewer.flatjson,
            index,
            viewer.mode,
            &viewer.layout,
            viewer.dimensions.width,
        );
        if viewer.layout.show_relative_line_numbers {
            line.line_number.relative = Some(delta_to_focused_row.unsigned_abs());
        }
        line.focused = focused;
        line.focused_because_matching_container_pair = focused_because_matching_container_pair;
        line.search_matches = Some(search_matches.clone());
        line.focused_search_match = focused_search_match;
        line.cached_truncated_value = Some(truncated_row_value_views.entry(index));

        let lines = line.print_line()?;

        *search_matches = line.search_matches.unwrap();

        Ok(lines)
    }

    fn line_primitive_value_ref<'a, 'b>(
        &'a self,
        row: &'a Row,
        viewer: &'b JsonViewer,
    ) -> Option<&'b str> {
        match &row.value {
            Value::OpenContainer { .. } | Value::CloseContainer { .. } => None,
            _ => {
                let range = row.range.clone();
                if let Value::String = &row.value {
                    Some(&viewer.flatjson.1[range.start + 1..range.end - 1])
                } else {
                    Some(&viewer.flatjson.1[range])
                }
            }
        }
    }

    fn print_status_bar_impl(
        &mut self,
        viewer: &JsonViewer,
        input_buffer: &[u8],
        input_filename: &str,
        search_state: &SearchState,
        message: &Option<(String, MessageSeverity)>,
    ) -> std::fmt::Result {
        self.terminal
            .position_cursor(1, self.dimensions.height - 1)?;
        self.terminal.clear_line()?;
        self.terminal.set_style(&terminal::Style {
            inverted: true,
            ..terminal::Style::default()
        })?;
        // Need to print a line to ensure the entire bar with the path to
        // the node and the filename is highlighted.
        for _ in 0..self.dimensions.width {
            self.terminal.write_char(' ')?;
        }
        self.terminal.write_char('\r')?;

        let path_to_node = viewer
            .flatjson
            .build_path_to_node(PathType::DotWithTopLevelIndex, viewer.focused_row)
            .unwrap();
        self.print_path_to_node_and_file_name(
            &path_to_node,
            input_filename,
            viewer.dimensions.width as isize,
        )?;

        self.terminal.position_cursor(1, self.dimensions.height)?;
        self.terminal.clear_line()?;

        if let Some((contents, severity)) = message {
            self.terminal.set_style(&terminal::Style {
                fg: severity.color(),
                ..terminal::Style::default()
            })?;
            self.terminal.write_str(contents)?;
        } else if search_state.showing_matches() {
            self.terminal
                .write_char(search_state.direction.prompt_char())?;
            self.terminal.write_str(&search_state.search_term)?;

            if let Some((match_num, just_wrapped)) = search_state.active_search_state() {
                // Print out which match we're on:
                let match_tracker = format!("[{}/{}]", match_num + 1, search_state.num_matches());
                self.terminal.position_cursor(
                    self.dimensions.width
                        - (1 + MAX_BUFFER_SIZE as u16)
                        - (3 + match_tracker.len() as u16 + 3),
                    self.dimensions.height,
                )?;

                let wrapped_char = if just_wrapped { 'W' } else { ' ' };
                write!(self.terminal, " {wrapped_char} {match_tracker}")?;
            }
        } else {
            write!(self.terminal, ":")?;
        }

        self.terminal.position_cursor(
            // TODO: This can overflow on very skinny screens (2-3 columns).
            self.dimensions.width - (1 + MAX_BUFFER_SIZE as u16),
            self.dimensions.height,
        )?;
        self.terminal
            .write_str(std::str::from_utf8(input_buffer).unwrap())?;

        // Position the cursor better for random debugging prints. (2 so it's after ':')
        self.terminal.position_cursor(2, self.dimensions.height)?;

        Ok(())
    }

    // input.data.viewer.gameDetail.plays[3].playStats[0].gsisPlayer.id filename.>
    // input.data.viewer.gameDetail.plays[3].playStats[0].gsisPlayer.id fi>
    // // Path also shrinks if needed
    // <.data.viewer.gameDetail.plays[3].playStats[0].gsisPlayer.id
    fn print_path_to_node_and_file_name(
        &mut self,
        path_to_node: &str,
        filename: &str,
        width: isize,
    ) -> std::fmt::Result {
        let base_len = PATH_BASE.len() as isize;
        let path_display_width = UnicodeWidthStr::width(path_to_node) as isize;
        let row = self.dimensions.height - 1;

        let space_available_for_filename =
            width - base_len - path_display_width - SPACE_BETWEEN_PATH_AND_FILENAME;
        let mut space_available_for_base = width - path_display_width;

        let inverted_style = terminal::Style {
            inverted: true,
            ..terminal::Style::default()
        };

        let truncated_filename =
            TruncatedStrView::init_start(filename, space_available_for_filename);

        if truncated_filename.any_contents_visible() {
            let filename_width = truncated_filename.used_space().unwrap();
            space_available_for_base -= filename_width - SPACE_BETWEEN_PATH_AND_FILENAME;
        }

        let truncated_base = TruncatedStrView::init_back(PATH_BASE, space_available_for_base);

        self.terminal.position_cursor(1, row)?;
        self.terminal.set_style(&inverted_style)?;
        self.terminal.set_bg(terminal::LIGHT_BLACK)?;

        let base_slice = TruncatedStrSlice {
            s: PATH_BASE,
            truncated_view: &truncated_base,
        };

        write!(self.terminal, "{base_slice}")?;

        self.terminal.set_bg(terminal::DEFAULT)?;

        // If the path is the exact same width as the screen, we won't print out anything
        // for the PATH_BASE, and the path won't be truncated. But there is truncated
        // content (the PATH_BASE), so we'll just manually handle this case.
        if truncated_base.used_space().is_none() && path_display_width == width {
            self.terminal.write_char('…')?;
            let mut graphemes = path_to_node.graphemes(true);
            // Skip one character.
            graphemes.next();
            self.terminal.write_str(graphemes.as_str())?;
        } else {
            let path_slice = TruncatedStrSlice {
                s: path_to_node,
                truncated_view: &TruncatedStrView::init_back(path_to_node, width),
            };

            write!(self.terminal, "{path_slice}")?;
        }

        if truncated_filename.any_contents_visible() {
            let filename_width = truncated_filename.used_space().unwrap();

            self.terminal
                .position_cursor(self.dimensions.width - (filename_width as u16) + 1, row)?;
            self.terminal.set_style(&inverted_style)?;

            let truncated_slice = TruncatedStrSlice {
                s: filename,
                truncated_view: &truncated_filename,
            };

            write!(self.terminal, "{truncated_slice}")?;
        }

        Ok(())
    }

    pub fn scroll_focused_line_right(&mut self, viewer: &JsonViewer, count: usize) {
        self.scroll_focused_line(viewer, count, true);
    }

    pub fn scroll_focused_line_left(&mut self, viewer: &JsonViewer, count: usize) {
        self.scroll_focused_line(viewer, count, false);
    }

    pub fn scroll_focused_line(&mut self, viewer: &JsonViewer, count: usize, to_right: bool) {
        let row = viewer.focused_row;
        let tsv = self.truncated_row_value_views.get(&row);
        if let Some(tsv) = tsv {
            if tsv.range.is_none() {
                return;
            }

            // Make tsv not a reference.
            let mut tsv = *tsv;
            let value_ref = self
                .line_primitive_value_ref(&viewer.flatjson[row], viewer)
                .unwrap();
            if to_right {
                tsv = tsv.scroll_right(value_ref, count);
            } else {
                tsv = tsv.scroll_left(value_ref, count);
            }
            self.truncated_row_value_views
                .insert(viewer.focused_row, tsv);
        }
    }

    pub fn scroll_focused_line_to_an_end(&mut self, viewer: &JsonViewer) {
        let row = viewer.focused_row;
        let tsv = self.truncated_row_value_views.get(&row);
        if let Some(tsv) = tsv {
            if tsv.range.is_none() {
                return;
            }

            // Make tsv not a reference.
            let mut tsv = *tsv;
            let value_ref = self
                .line_primitive_value_ref(&viewer.flatjson[row], viewer)
                .unwrap();
            tsv = tsv.jump_to_an_end(value_ref);
            self.truncated_row_value_views
                .insert(viewer.focused_row, tsv);
        }
    }

    pub fn scroll_line_to_search_match(
        &mut self,
        viewer: &JsonViewer,
        focused_search_range: Range<usize>,
    ) {
        let row = viewer.focused_row;
        let tsv = self.truncated_row_value_views.get(&row);
        if let Some(tsv) = tsv {
            // Make tsv not a reference.
            let mut tsv = *tsv;
            if tsv.range.is_none() {
                return;
            }

            let json_row = &viewer.flatjson[row];
            let value_ref = self.line_primitive_value_ref(json_row, viewer).unwrap();

            let mut range = json_row.range.clone();
            if json_row.is_string() {
                range.start += 1;
                range.end -= 1;
            }

            let no_overlap =
                focused_search_range.end <= range.start || range.end <= focused_search_range.start;
            if no_overlap {
                return;
            }

            let mut value_range_start = range.start;
            if let Value::String = &json_row.value {
                value_range_start += 1;
            }

            let offset_focused_range = Range {
                start: focused_search_range.start.saturating_sub(value_range_start),
                end: focused_search_range.end - value_range_start,
            };

            tsv = tsv.focus(value_ref, &offset_focused_range);

            self.truncated_row_value_views
                .insert(viewer.focused_row, tsv);
        }
    }
}

// Places the '\n'-separated lines of a printed row on consecutive screen
// rows, dropping the first lines_to_skip lines and any after max_lines,
// along with their cursor and style changes.
struct ClippedTerminal<'a> {
    inner: &'a mut dyn Terminal,
    line: usize,
    lines_to_skip: usize,
    first_screen_row: u16,
    max_lines: usize,
}

impl ClippedTerminal<'_> {
    fn start(&mut self) -> std::fmt::Result {
        self.move_to_line()
    }

    fn is_visible(&self) -> bool {
        (self.lines_to_skip..self.lines_to_skip + self.max_lines).contains(&self.line)
    }

    // Moving onto a line resets the style, so dropped style changes don't matter.
    fn move_to_line(&mut self) -> std::fmt::Result {
        let screen_row =
            self.first_screen_row + self.line.saturating_sub(self.lines_to_skip) as u16;
        self.forward(|t| {
            t.position_cursor(1, screen_row)?;
            t.clear_line()
        })
    }

    fn forward(
        &mut self,
        f: impl FnOnce(&mut dyn Terminal) -> std::fmt::Result,
    ) -> std::fmt::Result {
        if self.is_visible() {
            f(self.inner)
        } else {
            Ok(())
        }
    }
}

impl Write for ClippedTerminal<'_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        for (i, part) in s.split('\n').enumerate() {
            if i > 0 {
                self.line += 1;
                self.move_to_line()?;
            }
            if !part.is_empty() {
                self.forward(|t| t.write_str(part))?;
            }
        }
        Ok(())
    }
}

impl Terminal for ClippedTerminal<'_> {
    fn clear_line(&mut self) -> std::fmt::Result {
        self.forward(|t| t.clear_line())
    }

    fn position_cursor(&mut self, col: u16, row: u16) -> std::fmt::Result {
        self.forward(|t| t.position_cursor(col, row))
    }

    fn position_cursor_col(&mut self, col: u16) -> std::fmt::Result {
        self.forward(|t| t.position_cursor_col(col))
    }

    fn set_style(&mut self, style: &Style) -> std::fmt::Result {
        self.forward(|t| t.set_style(style))
    }

    fn reset_style(&mut self) -> std::fmt::Result {
        self.forward(|t| t.reset_style())
    }

    fn set_fg(&mut self, color: Color) -> std::fmt::Result {
        self.forward(|t| t.set_fg(color))
    }

    fn set_bg(&mut self, color: Color) -> std::fmt::Result {
        self.forward(|t| t.set_bg(color))
    }

    fn set_inverted(&mut self, inverted: bool) -> std::fmt::Result {
        self.forward(|t| t.set_inverted(inverted))
    }

    fn set_bold(&mut self, bold: bool) -> std::fmt::Result {
        self.forward(|t| t.set_bold(bold))
    }

    fn set_dimmed(&mut self, dimmed: bool) -> std::fmt::Result {
        self.forward(|t| t.set_dimmed(dimmed))
    }

    #[cfg(test)]
    fn output(&self) -> &str {
        self.inner.output()
    }

    #[cfg(test)]
    fn clear_output(&mut self) {
        self.inner.clear_output()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::test::VisibleEscapesTerminal;

    #[test]
    fn test_clipped_terminal_places_visible_lines_on_screen_rows() -> std::fmt::Result {
        let mut inner = VisibleEscapesTerminal::new(true, true);
        let mut terminal = ClippedTerminal {
            inner: &mut inner,
            line: 0,
            lines_to_skip: 1,
            first_screen_row: 5,
            max_lines: 2,
        };
        terminal.start()?;
        terminal.set_bold(true)?;
        write!(terminal, "a")?;
        terminal.reset_style()?;
        write!(terminal, "\nb\nc")?;
        terminal.position_cursor_col(10)?;
        write!(terminal, "\nd")?;
        terminal.reset_style()?;

        // Styles of dropped lines are dropped too.
        assert_eq!("_RC(1,5)_b_RC(1,6)_c_C(10)_", terminal.output());

        Ok(())
    }
}
