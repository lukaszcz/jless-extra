use std::fmt::{Result, Write};
use std::io;
use std::ops::Range;
use std::time::{Duration, Instant};

use crate::options::ThemeOption;

const BACKGROUND_QUERY: &[u8] = b"\x1b]11;?\x1b\\";
const BACKGROUND_RESPONSE_PREFIX: &[u8] = b"\x1b]11;";
const BACKGROUND_QUERY_TIMEOUT: Duration = Duration::from_millis(150);
const MAX_BACKGROUND_RESPONSE_BYTES: usize = 256;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Color {
    C16(u8),
    Default,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Theme {
    Dark,
    Light,
}

// Commented out colors are unused.
// #[cfg(test)]
// pub const BLACK: Color = Color::C16(0);
pub const RED: Color = Color::C16(1);
pub const GREEN: Color = Color::C16(2);
pub const YELLOW: Color = Color::C16(3);
pub const BLUE: Color = Color::C16(4);
pub const MAGENTA: Color = Color::C16(5);
// pub const CYAN: Color = Color::C16(6);
pub const WHITE: Color = Color::C16(7);
pub const LIGHT_BLACK: Color = Color::C16(8);
// pub const LIGHT_RED: Color = Color::C16(9);
// pub const LIGHT_GREEN: Color = Color::C16(10);
// pub const LIGHT_YELLOW: Color = Color::C16(11);
pub const LIGHT_BLUE: Color = Color::C16(12);
// pub const LIGHT_MAGENTA: Color = Color::C16(13);
// pub const LIGHT_CYAN: Color = Color::C16(14);
// pub const LIGHT_WHITE: Color = Color::C16(15);
pub const DEFAULT: Color = Color::Default;

#[derive(Copy, Clone)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub inverted: bool,
    pub bold: bool,
    pub dimmed: bool,
}

impl Style {
    pub const fn default() -> Self {
        Style {
            fg: Color::Default,
            bg: Color::Default,
            inverted: false,
            bold: false,
            dimmed: false,
        }
    }
}

impl Default for Style {
    fn default() -> Self {
        Style::default()
    }
}

pub trait Terminal: Write {
    fn clear_line(&mut self) -> Result;

    fn position_cursor(&mut self, col: u16, row: u16) -> Result;
    fn position_cursor_col(&mut self, col: u16) -> Result;

    fn set_style(&mut self, style: &Style) -> Result;
    fn reset_style(&mut self) -> Result;

    fn set_fg(&mut self, color: Color) -> Result;
    fn set_bg(&mut self, color: Color) -> Result;
    fn set_inverted(&mut self, inverted: bool) -> Result;
    fn set_bold(&mut self, bold: bool) -> Result;
    fn set_dimmed(&mut self, dimmed: bool) -> Result;

    #[cfg(test)]
    fn output(&self) -> &str;

    #[cfg(test)]
    fn clear_output(&mut self);
}

pub struct AnsiTerminal {
    pub output: String,
    pub style: Style,
    theme: Theme,
}

impl AnsiTerminal {
    pub fn new(output: String) -> Self {
        AnsiTerminal {
            output,
            style: Style::default(),
            theme: Theme::Dark,
        }
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    pub fn flush_contents<W: std::io::Write>(&mut self, out: &mut W) -> std::io::Result<usize> {
        let bytes = out.write(self.output.as_bytes())?;
        out.flush()?;
        self.output.clear();
        Ok(bytes)
    }
}

impl Write for AnsiTerminal {
    fn write_str(&mut self, s: &str) -> Result {
        self.output.write_str(s)
    }
}

impl Terminal for AnsiTerminal {
    fn clear_line(&mut self) -> Result {
        write!(self, "\x1b[2K")
    }

    fn position_cursor(&mut self, col: u16, row: u16) -> Result {
        write!(self, "\x1b[{row};{col}H")?;
        self.reset_style()
    }

    fn position_cursor_col(&mut self, col: u16) -> Result {
        write!(self, "\x1b[{col}G")?;
        self.reset_style()
    }

    fn set_style(&mut self, style: &Style) -> Result {
        self.set_fg(style.fg)?;
        self.set_bg(style.bg)?;
        self.set_inverted(style.inverted)?;
        self.set_bold(style.bold)?;
        self.set_dimmed(style.dimmed)?;
        Ok(())
    }

    fn reset_style(&mut self) -> Result {
        self.style = Style::default();
        write!(self, "\x1b[0m")
    }

    fn set_fg(&mut self, color: Color) -> Result {
        if self.style.fg != color {
            match color_for_theme(color, self.theme, true) {
                Color::C16(c) => write!(self, "\x1b[38;5;{c}m")?,
                Color::Default => write!(self, "\x1b[39m")?,
            }
            self.style.fg = color;
        }
        Ok(())
    }

    fn set_bg(&mut self, color: Color) -> Result {
        if self.style.bg != color {
            match color_for_theme(color, self.theme, false) {
                Color::C16(c) => write!(self, "\x1b[48;5;{c}m")?,
                Color::Default => write!(self, "\x1b[49m")?,
            }
            self.style.bg = color;
        }
        Ok(())
    }

    fn set_inverted(&mut self, inverted: bool) -> Result {
        if self.style.inverted != inverted {
            if inverted {
                write!(self, "\x1b[7m")?;
            } else {
                write!(self, "\x1b[27m")?;
            }
            self.style.inverted = inverted;
        }
        Ok(())
    }

    fn set_bold(&mut self, bold: bool) -> Result {
        if self.style.bold != bold {
            if bold {
                write!(self, "\x1b[1m")?;
            } else {
                write!(self, "\x1b[22m")?;
                // Also resets dimmed, so set that if we need to
                if self.style.dimmed {
                    write!(self, "\x1b[2m")?;
                }
            }
            self.style.bold = bold;
        }
        Ok(())
    }

    fn set_dimmed(&mut self, dimmed: bool) -> Result {
        if self.style.dimmed != dimmed {
            if dimmed {
                write!(self, "\x1b[2m")?;
            } else {
                write!(self, "\x1b[22m")?;
                // Also resets bold, so set that if we need to
                if self.style.bold {
                    write!(self, "\x1b[1m")?;
                }
            }
            self.style.dimmed = dimmed;
        }
        Ok(())
    }

    #[cfg(test)]
    fn output(&self) -> &str {
        &self.output
    }

    #[cfg(test)]
    fn clear_output(&mut self) {
        self.output.clear()
    }
}

fn color_for_theme(color: Color, theme: Theme, foreground: bool) -> Color {
    if theme == Theme::Dark {
        return color;
    }

    match (foreground, color) {
        (true, Color::C16(1)) => Color::C16(124),
        (true, Color::C16(2)) => Color::C16(28),
        (true, Color::C16(3)) => Color::C16(136),
        (true, Color::C16(4 | 12)) => Color::C16(25),
        (true, Color::C16(5)) => Color::C16(90),
        (true, Color::C16(7)) => Color::C16(16),
        (true, Color::C16(8)) => Color::C16(240),
        (false, Color::C16(4)) => Color::C16(117),
        (false, Color::C16(8)) => Color::C16(244),
        (_, color) => color,
    }
}

pub fn resolve_theme(theme_option: ThemeOption) -> (Theme, Vec<u8>) {
    match theme_option {
        ThemeOption::Dark => (Theme::Dark, Vec::new()),
        ThemeOption::Light => (Theme::Light, Vec::new()),
        ThemeOption::Auto => {
            let (detected_theme, pending_input) = query_terminal_background();
            let theme = detected_theme
                .or_else(|| {
                    std::env::var("COLORFGBG")
                        .ok()
                        .and_then(|value| theme_from_colorfgbg(&value))
                })
                .unwrap_or(Theme::Dark);
            (theme, pending_input)
        }
    }
}

fn query_terminal_background() -> (Option<Theme>, Vec<u8>) {
    use std::io::Write as _;

    let mut stdout = io::stdout().lock();
    if stdout
        .write_all(BACKGROUND_QUERY)
        .and_then(|()| stdout.flush())
        .is_err()
    {
        return (None, Vec::new());
    }
    drop(stdout);

    let start = Instant::now();
    let mut received = Vec::new();

    while start.elapsed() < BACKGROUND_QUERY_TIMEOUT
        && received.len() < MAX_BACKGROUND_RESPONSE_BYTES
    {
        let remaining = BACKGROUND_QUERY_TIMEOUT.saturating_sub(start.elapsed());
        let timeout = remaining.as_millis().clamp(1, i32::MAX as u128) as i32;
        let mut poll_fd = libc::pollfd {
            fd: libc::STDIN_FILENO,
            events: libc::POLLIN,
            revents: 0,
        };

        // SAFETY: poll_fd is a valid pointer to one pollfd, and the descriptor is stdin.
        let poll_result = unsafe { libc::poll(&mut poll_fd, 1, timeout) };
        if poll_result == 0 {
            break;
        }
        if poll_result < 0 {
            if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            }
            break;
        }
        if poll_fd.revents & (libc::POLLIN | libc::POLLHUP) == 0 {
            break;
        }

        let mut byte = 0;
        // SAFETY: byte is writable for one byte, and stdin is a valid descriptor.
        let bytes_read =
            unsafe { libc::read(libc::STDIN_FILENO, std::ptr::addr_of_mut!(byte).cast(), 1) };
        if bytes_read != 1 {
            break;
        }
        received.push(byte);

        if let Some(theme) = remove_background_response(&mut received) {
            return (theme, received);
        }
    }

    (None, received)
}

fn background_response(input: &[u8]) -> Option<(Range<usize>, Option<Theme>)> {
    let start = input
        .windows(BACKGROUND_RESPONSE_PREFIX.len())
        .position(|window| window == BACKGROUND_RESPONSE_PREFIX)?;
    let content_start = start + BACKGROUND_RESPONSE_PREFIX.len();
    let (end, terminator_len) =
        (content_start..input.len()).find_map(|index| match input[index] {
            b'\x07' => Some((index, 1)),
            b'\x1b' if input.get(index + 1) == Some(&b'\\') => Some((index, 2)),
            _ => None,
        })?;
    let theme = std::str::from_utf8(&input[content_start..end])
        .ok()
        .and_then(theme_from_rgb);
    Some((start..end + terminator_len, theme))
}

fn remove_background_response(input: &mut Vec<u8>) -> Option<Option<Theme>> {
    let (range, theme) = background_response(input)?;
    input.drain(range);
    Some(theme)
}

fn theme_from_rgb(rgb: &str) -> Option<Theme> {
    let components = rgb.strip_prefix("rgb:")?.split('/').collect::<Vec<_>>();
    if components.len() != 3 {
        return None;
    }
    let [red, green, blue] = [
        component_to_linear(components[0])?,
        component_to_linear(components[1])?,
        component_to_linear(components[2])?,
    ];
    let luminance = 0.2126 * red + 0.7152 * green + 0.0722 * blue;
    Some(if luminance >= 0.5 {
        Theme::Light
    } else {
        Theme::Dark
    })
}

fn component_to_linear(component: &str) -> Option<f64> {
    if component.is_empty() || component.len() > 4 {
        return None;
    }
    let value = u32::from_str_radix(component, 16).ok()? as f64;
    let max_value = ((1u32 << (component.len() * 4)) - 1) as f64;
    let srgb = value / max_value;
    Some(if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    })
}

fn theme_from_colorfgbg(colorfgbg: &str) -> Option<Theme> {
    match colorfgbg.rsplit(';').next()?.parse::<u8>().ok()? {
        0..=6 => Some(Theme::Dark),
        7 | 15 => Some(Theme::Light),
        _ => None,
    }
}

#[cfg(test)]
mod theme_tests {
    use super::{AnsiTerminal, BLUE, GREEN, Terminal, Theme};

    #[test]
    fn light_theme_maps_semantic_colors_to_light_palette() {
        let mut terminal = AnsiTerminal::new(String::new());
        terminal.set_theme(Theme::Light);

        terminal.set_fg(GREEN).unwrap();
        terminal.set_bg(BLUE).unwrap();

        assert_eq!(terminal.output, "\x1b[38;5;28m\x1b[48;5;117m");
    }

    #[test]
    fn background_color_response_detects_light_and_dark_themes() {
        assert_eq!(
            super::theme_from_rgb("rgb:ffff/ffff/ffff"),
            Some(Theme::Light)
        );
        assert_eq!(
            super::theme_from_rgb("rgb:0000/0000/0000"),
            Some(Theme::Dark)
        );
    }

    #[test]
    fn background_query_bytes_are_removed_without_losing_user_input() {
        let mut input = b"x\x1b]11;rgb:ffff/ffff/ffff\x07y".to_vec();

        let theme = super::remove_background_response(&mut input);

        assert_eq!(theme, Some(Some(Theme::Light)));
        assert_eq!(input, b"xy");
    }

    #[test]
    fn colorfgbg_fallback_detects_light_and_dark_backgrounds() {
        assert_eq!(super::theme_from_colorfgbg("15;0"), Some(Theme::Dark));
        assert_eq!(super::theme_from_colorfgbg("0;15"), Some(Theme::Light));
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    const COLOR_NAMES: [&str; 16] = [
        "Black",
        "Red",
        "Green",
        "Yellow",
        "Blue",
        "Magenta",
        "Cyan",
        "White",
        "LightBlack",
        "LightRed",
        "LightGreen",
        "LightYellow",
        "LightBlue",
        "LightMagenta",
        "LightCyan",
        "LightWhite",
    ];

    impl std::fmt::Display for Color {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Color::C16(c) => write!(f, "{}", COLOR_NAMES.get(*c as usize).unwrap_or(&"?")),
                Color::Default => write!(f, "Default"),
            }
        }
    }

    pub struct TextOnlyTerminal {
        pub output: String,
    }

    impl TextOnlyTerminal {
        pub fn new() -> Self {
            TextOnlyTerminal {
                output: String::new(),
            }
        }
    }

    impl Write for TextOnlyTerminal {
        fn write_str(&mut self, s: &str) -> Result {
            self.output.write_str(s)
        }
    }

    #[rustfmt::skip]
    impl Terminal for TextOnlyTerminal {
        fn clear_line(&mut self) -> Result { Ok(()) }
        fn position_cursor(&mut self, _row: u16, _col: u16) -> Result { Ok(()) }
        fn position_cursor_col(&mut self, _col: u16) -> Result { Ok(()) }
        fn set_style(&mut self, _style: &Style) -> Result { Ok(()) }
        fn reset_style(&mut self) -> Result { Ok(()) }
        fn set_fg(&mut self, _color: Color) -> Result { Ok(()) }
        fn set_bg(&mut self, _color: Color) -> Result { Ok(()) }
        fn set_inverted(&mut self, _inverted: bool) -> Result { Ok(()) }
        fn set_bold(&mut self, _bold: bool) -> Result { Ok(()) }
        fn set_dimmed(&mut self, _bold: bool) -> Result { Ok(()) }
        fn output(&self) -> &str { &self.output }
        fn clear_output(&mut self) { self.output.clear() }
    }

    pub struct VisibleEscapesTerminal {
        pub output: String,
        pub style: Style,
        pub pending_style: Style,
        pub show_position: bool,
        pub show_style: bool,
    }

    impl VisibleEscapesTerminal {
        pub fn new(show_position: bool, show_style: bool) -> Self {
            VisibleEscapesTerminal {
                output: String::new(),
                style: Style::default(),
                pending_style: Style::default(),
                show_position,
                show_style,
            }
        }
    }

    impl VisibleEscapesTerminal {
        fn write_pending_styles(&mut self) -> Result {
            if self.show_style {
                if self.style.fg != self.pending_style.fg {
                    write!(self.output, "_FG({})_", self.pending_style.fg)?;
                }
                if self.style.bg != self.pending_style.bg {
                    write!(self.output, "_BG({})_", self.pending_style.bg)?;
                }
                if self.style.inverted != self.pending_style.inverted {
                    if self.pending_style.inverted {
                        write!(self.output, "_INV_")?;
                    } else {
                        write!(self.output, "_!INV_")?;
                    }
                }
                if self.style.bold != self.pending_style.bold {
                    if self.pending_style.bold {
                        write!(self.output, "_B_")?;
                    } else {
                        write!(self.output, "_!B_")?;
                    }
                }
                if self.style.dimmed != self.pending_style.dimmed {
                    if self.pending_style.dimmed {
                        write!(self.output, "_D_")?;
                    } else {
                        write!(self.output, "_!D_")?;
                    }
                }
            }

            self.style = self.pending_style;

            Ok(())
        }
    }

    impl Write for VisibleEscapesTerminal {
        fn write_str(&mut self, s: &str) -> Result {
            self.write_pending_styles()?;
            self.output.write_str(s)
        }
    }

    impl Terminal for VisibleEscapesTerminal {
        fn clear_line(&mut self) -> Result {
            Ok(())
        }

        fn position_cursor(&mut self, row: u16, col: u16) -> Result {
            if self.show_position {
                write!(self, "_RC({row},{col})_")
            } else {
                Ok(())
            }
        }

        fn position_cursor_col(&mut self, col: u16) -> Result {
            if self.show_position {
                write!(self, "_C({col})_")
            } else {
                Ok(())
            }
        }

        fn set_style(&mut self, style: &Style) -> Result {
            self.pending_style = *style;
            Ok(())
        }

        fn reset_style(&mut self) -> Result {
            self.style = Style::default();
            self.pending_style = Style::default();
            if self.show_style {
                write!(self, "_R_")?;
            }
            Ok(())
        }

        fn set_fg(&mut self, color: Color) -> Result {
            self.pending_style.fg = color;
            Ok(())
        }

        fn set_bg(&mut self, color: Color) -> Result {
            self.pending_style.bg = color;
            Ok(())
        }

        fn set_inverted(&mut self, inverted: bool) -> Result {
            self.pending_style.inverted = inverted;
            Ok(())
        }

        fn set_bold(&mut self, bold: bool) -> Result {
            self.pending_style.bold = bold;
            Ok(())
        }

        fn set_dimmed(&mut self, dimmed: bool) -> Result {
            self.pending_style.dimmed = dimmed;
            Ok(())
        }

        fn output(&self) -> &str {
            &self.output
        }

        fn clear_output(&mut self) {
            self.style = Style::default();
            self.pending_style = Style::default();
            self.output.clear()
        }
    }
}
