use std::path::PathBuf;

use clap::{ArgAction, Parser, ValueEnum};

use crate::viewer::Mode;

#[derive(PartialEq, Eq, Copy, Clone, Debug, ValueEnum)]
pub enum DataFormat {
    Json,
    Yaml,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, ValueEnum)]
pub enum ThemeOption {
    Auto,
    Dark,
    Light,
}

/// A pager for JSON (or YAML) data
#[derive(Debug, Parser)]
#[command(name = "jless", version)]
pub struct Opt {
    /// Input file. jless will read from stdin if no input file is
    /// provided, or '-' is specified. If a filename is provided, jless
    /// will check the extension to determine what the input format is,
    /// and by default will assume JSON. Can specify input format
    /// explicitly using --json or --yaml.
    pub input: Option<PathBuf>,

    /// Initial viewing mode. In line mode (--mode line), opening
    /// and closing curly and square brackets are shown and all
    /// Object keys are quoted. In data mode (--mode data; the default),
    /// closing braces, commas, and quotes around Object keys are elided.
    /// The active mode can be toggled by pressing 'm'.
    #[arg(short, long, value_enum, hide_possible_values = true, default_value_t = Mode::Data)]
    pub mode: Mode,

    /// Color theme. Auto detects the terminal background. The JLESS_THEME
    /// environment variable can be used instead of this option.
    #[arg(long, value_enum, env = "JLESS_THEME", default_value = "auto")]
    pub theme: ThemeOption,

    // This godforsaken configuration to get both --line-numbers and --no-line-numbers to
    // work (with --line-numbers as the default) and --relative-line-numbers and
    // --no-relative-line-numbers to work (with --no-relative-line-numbers as the default)
    // was taken from here:
    //
    // https://jwodder.github.io/kbits/posts/clap-bool-negate/
    /// Don't show line numbers.
    #[arg(short = 'N', long = "no-line-numbers", action = ArgAction::SetFalse)]
    pub show_line_numbers: bool,

    /// Show "line" numbers (default). Line numbers are determined by
    /// the line number of a given line if the document were pretty printed.
    /// These means there are discontinuities when viewing in data mode
    /// because the lines containing closing brackets and braces aren't displayed.
    #[arg(
        short = 'n',
        long = "line-numbers",
        overrides_with = "show_line_numbers"
    )]
    pub _show_line_numbers_hidden: bool,

    /// Show the line number relative to the currently focused line. Relative line
    /// numbers help you use a count with vertical motion commands (j k) without
    /// having to count.
    #[arg(
        short = 'r',
        long = "relative-line-numbers",
        overrides_with = "_show_relative_line_numbers_hidden"
    )]
    pub show_relative_line_numbers: bool,

    /// Don't show relative line numbers (default).
    #[arg(short = 'R', long = "no-relative-line-numbers")]
    _show_relative_line_numbers_hidden: bool,

    /// Don't wrap long values; truncate them instead. Truncated values can
    /// be scrolled horizontally.
    #[arg(long = "no-wrap", action = ArgAction::SetFalse)]
    pub wrap: bool,

    /// Wrap long values onto multiple lines (default), so they can always
    /// be seen in full.
    #[arg(long = "wrap", overrides_with = "wrap")]
    pub _wrap_hidden: bool,

    /// Number of lines to maintain as padding between the currently
    /// focused row and the top or bottom of the screen. Setting this to
    /// a large value will keep the focused in the middle of the screen
    /// (except at the start or end of a file).
    #[arg(long = "scrolloff", default_value_t = 3)]
    pub scrolloff: u16,

    /// Parse input as JSON, regardless of file extension.
    #[arg(long = "json", group = "data-format", display_order = 1000)]
    pub json: bool,

    /// Parse input as YAML, regardless of file extension.
    #[arg(long = "yaml", group = "data-format", display_order = 1000)]
    pub yaml: bool,
}

impl Opt {
    pub fn data_format(&self) -> Option<DataFormat> {
        if self.json {
            Some(DataFormat::Json)
        } else if self.yaml {
            Some(DataFormat::Yaml)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use clap::{CommandFactory, Parser};

    use super::{Opt, ThemeOption};

    #[test]
    fn theme_can_be_selected_from_command_line() {
        for (value, expected) in [
            ("light", ThemeOption::Light),
            ("dark", ThemeOption::Dark),
            ("auto", ThemeOption::Auto),
        ] {
            let options = Opt::try_parse_from(["jless", "--theme", value]).unwrap();
            assert_eq!(options.theme, expected);
        }
    }

    #[test]
    fn wrap_is_default_and_can_be_disabled() {
        for (args, expected) in [
            (vec!["jless"], true),
            (vec!["jless", "--no-wrap"], false),
            (vec!["jless", "--no-wrap", "--wrap"], true),
        ] {
            let options = Opt::try_parse_from(&args).unwrap();
            assert_eq!(options.wrap, expected, "{args:?}");
        }
    }

    #[test]
    fn theme_uses_jless_theme_environment_variable() {
        let has_environment_variable = Opt::command()
            .get_arguments()
            .any(|argument| argument.get_env() == Some(OsStr::new("JLESS_THEME")));

        assert!(has_environment_variable);
    }
}
