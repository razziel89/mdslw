/* An opinionated line wrapper for markdown files.
Copyright (C) 2023  Torsten Long

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use clap::{Parser, ValueEnum, builder::OsStr};
use clap_complete::Shell;
use serde::{Deserialize, Serialize};

// Command-line interface definition.

/// A generic value that knows its origin. That is, we use the "Default" variant when defining
/// default values in the CliArgs struct but we always parse to the "Parsed" variant when parsing
/// from a command line argument. That way, we can distinguish whether an option has been provided
/// on the command line or was taken as a default.
///
/// Note that default_value_t will perform a display-then-parse-again round trip, which means it
/// actually does not matter whether we use the "Parsed" or the "Default" variant in the
/// default_value_t bit. However, we explicitly add a zero-width space to the end of every default
/// value to be able to determine whether teh value is a default. Note that that will result in
/// unexpected behaviour if a user ever adds such a character to the end of an argument, but what
/// can you do. It's either that, or replacing clap, or not having config file support. In my view,
/// config file support is worth this work-around.
#[derive(Clone, Debug)]
pub enum ValueWOrigin<T> {
    Default(T),
    Parsed(T),
}

impl<T> ValueWOrigin<T> {
    // All default values that can also come from config files will end in this character. It is the
    // UTF8 zero-width space. All terminals that I tested do not display that character, but it is
    // present in the internal default string. We append that character to every default value that
    // can also come from a config file. That way, we can actually determine whether a value is a
    // default or not. See the Implementation of FromStr for this struct.
    const ZWS: char = '\u{200b}';
    const ZWS_LEN: usize = Self::ZWS.len_utf8();

    /// Get the correct value with the following precedence:
    ///   - If we contain a "Parsed", return the value contained in it. The user has specified that
    ///     on the command line, which means it takes precedence.
    ///   - If we contain a "Default" and the other value contains a "Some", return that.
    ///     That means the user has not specified that option on the command line, but a config file
    ///     contains it.
    ///   - Otherwise, return the value in the "Default".
    ///     In that case, neither has the user specified that option on the command line, nor is it
    ///     contained in any config file.
    fn resolve(&self, other: Option<T>) -> T
    where
        T: Clone,
    {
        match self {
            ValueWOrigin::Parsed(val) => val.clone(),
            ValueWOrigin::Default(val) => other.unwrap_or_else(|| val.clone()),
        }
    }
}

impl<T> FromStr for ValueWOrigin<T>
where
    T: FromStr,
{
    type Err = <T as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.ends_with(Self::ZWS) {
            match s[..s.len() - Self::ZWS_LEN].parse::<T>() {
                Ok(val) => Ok(Self::Default(val)),
                Err(err) => Err(err),
            }
        } else {
            match s.parse::<T>() {
                Ok(val) => Ok(Self::Parsed(val)),
                Err(err) => Err(err),
            }
        }
    }
}

impl<T> fmt::Display for ValueWOrigin<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueWOrigin::Parsed(val) | ValueWOrigin::Default(val) => {
                write!(f, "{}", val)
            }
        }
    }
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum OpMode {
    Both,
    Check,
    Format,
}

#[derive(Serialize, Deserialize, Copy, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Case {
    Ignore,
    Keep,
}

impl FromStr for Case {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "keep" => Ok(Self::Keep),
            "ignore" => Ok(Self::Ignore),
            _ => Err(String::from("possible values: ignore, keep")),
        }
    }
}

impl fmt::Display for Case {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ignore => {
                write!(f, "ignore")
            }
            Self::Keep => {
                write!(f, "keep")
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum ReportMode {
    None,
    Changed,
    State,
    DiffMyers,
    DiffPatience,
    DiffLcs,
}

impl ReportMode {
    pub fn is_diff_mode(&self) -> bool {
        self == &ReportMode::DiffMyers
            || self == &ReportMode::DiffPatience
            || self == &ReportMode::DiffLcs
    }
}

#[derive(Serialize, Deserialize, Copy, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum LinkActions {
    None,
    OutsourceInline,
    CollateDefs,
    Both,
}

impl FromStr for LinkActions {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(Self::None),
            "outsource-inline" => Ok(Self::OutsourceInline),
            "collate-defs" => Ok(Self::CollateDefs),
            "both" => Ok(Self::Both),
            _ => Err(String::from("possible values: none, outsource-inline, collate-defs, both")),
        }
    }
}

impl fmt::Display for LinkActions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::OutsourceInline => write!(f, "outsource-inline"),
            Self::CollateDefs => write!(f, "collate-defs"),
            Self::Both => write!(f, "both"),
        }
    }
}

#[derive(Serialize, Deserialize, Copy, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum KeepWhitespace {
    None,
    InLinks,
    Linebreaks,
    Both,
}

impl FromStr for KeepWhitespace {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(Self::None),
            "in-links" => Ok(Self::InLinks),
            "linebreaks" => Ok(Self::Linebreaks),
            "both" => Ok(Self::Both),
            _ => Err(String::from("possible values: none, in-links, linebreaks, both")),
        }
    }
}

impl fmt::Display for KeepWhitespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::InLinks => write!(f, "in-links"),
            Self::Linebreaks => write!(f, "linebreaks"),
            Self::Both => write!(f, "both"),
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    /// Paths to files or directories that shall be processed.
    pub paths: Vec<PathBuf>,
    /// The maximum line width that is acceptable. A value of 0 disables wrapping of{n}   long
    /// lines.
    #[arg(
        short = 'w',
        long,
        env = "MDSLW_MAX_WIDTH",
        default_value = "80\u{200b}"
    )]
    pub max_width: ValueWOrigin<usize>,
    /// A set of characters that are acceptable end of sentence markers.
    #[arg(short, long, env = "MDSLW_END_MARKERS", default_value = "?!:.\u{200b}")]
    pub end_markers: ValueWOrigin<String>,
    /// Mode of operation: "check" means exit with error if format has to be adjusted but do not
    /// format,{n}   "format" means format the file and exit with error in case of problems only,
    /// "both" means do both{n}   (useful as pre-commit hook).
    #[arg(value_enum, short, long, env = "MDSLW_MODE", default_value_t = OpMode::Format)]
    pub mode: OpMode,
    /// A space-separated list of languages whose suppression words as specified by unicode should
    /// be {n}   taken into account. See here for all languages:
    /// {n}   https://github.com/unicode-org/cldr-json/tree/main/cldr-json/cldr-segments-full/segments
    /// {n}   Use "none" to disable.
    /// Supported languages are: de en es fr it. Use "ac" for "author's choice",{n}   a list
    /// for the Enlish language defined by this tool's author.
    #[arg(short, long, env = "MDSLW_LANG", default_value = "ac\u{200b}")]
    pub lang: ValueWOrigin<String>,
    /// Space-separated list of words that end in one of END_MARKERS but that should not be
    /// followed by a line{n}   break. This is in addition to what is specified via --lang.
    #[arg(short, long, env = "MDSLW_SUPPRESSIONS", default_value = "\u{200b}")]
    pub suppressions: ValueWOrigin<String>,
    /// Space-separated list of words that end in one of END_MARKERS and that should be
    /// removed{n}   from the list of suppressions.
    #[arg(short, long, env = "MDSLW_IGNORES", default_value = "\u{200b}")]
    pub ignores: ValueWOrigin<String>,
    /// Specify an upstream auto-formatter that reads from stdin and writes to stdout.
    /// {n}   It will be called before mdslw will run. Useful if you want to chain multiple
    /// tools.{n}   Specify the command that will be executed. For example, specify "prettier"
    /// to{n}   call prettier first.
    /// The upstream auto-formatter runs in each file's directory if PATHS are{n}   specified.
    #[arg(long, env = "MDSLW_UPSTREAM_COMMAND", default_value = "\u{200b}")]
    pub upstream_command: ValueWOrigin<String>,
    /// Specify the arguments for the upstream auto-formatter. If --upstream-command is not set,
    /// {n}   the first word will be used as command. For example, with
    /// --upstream-command="prettier",{n}   set --upstream="--parser=markdown" to enable markdown
    /// parsing.
    #[arg(short, long, env = "MDSLW_UPSTREAM", default_value = "\u{200b}")]
    pub upstream: ValueWOrigin<String>,
    /// Specify a string that will be used to separate the value passed to --upstream into words.
    /// {n}   If empty, splitting is based on whitespace.
    #[arg(long, env = "MDSLW_UPSTREAM_SEPARATOR", default_value = "\u{200b}")]
    pub upstream_separator: ValueWOrigin<String>,
    /// How to handle the case of provided suppression words, both via --lang
    /// and{n}   --suppressions. Possible values: ignore, keep
    #[arg(short, long, env = "MDSLW_CASE", default_value = "ignore\u{200b}")]
    pub case: ValueWOrigin<Case>,
    /// The file extension used to find markdown files when an entry in{n}   PATHS is a directory.
    #[arg(long, env = "MDSLW_EXTENSION", default_value_t = String::from(".md"))]
    pub extension: String,
    /// Link actions to perform: "outsource-inline" to replace inline links by named links,
    /// {n}   "collate-defs" to gather all link definitions at the end of the document,
    /// {n}   "both" to do both, "none" to disable all link actions.
    /// {n}  .
    #[arg(long, env = "MDSLW_LINK_ACTIONS", default_value = "none\u{200b}")]
    pub link_actions: ValueWOrigin<LinkActions>,
    /// Whitespace preservation: "in-links" to keep spaces in link texts,
    /// {n}   "linebreaks" to keep existing linebreaks during line-wrapping,
    /// {n}   "both" to enable both, "none" to disable all whitespace preservation.
    /// {n}  .
    #[arg(long, env = "MDSLW_KEEP_WHITESPACE", default_value = "none\u{200b}")]
    pub keep_whitespace: ValueWOrigin<KeepWhitespace>,
    /// Format text in block quotes.
    /// {n}  .
    #[arg(long, env = "MDSLW_FORMAT_BLOCK_QUOTES", default_value = "false\u{200b}")]
    pub format_block_quotes: ValueWOrigin<bool>,
    /// Output shell completion file for the given shell to stdout and exit.{n}  .
    #[arg(value_enum, long, env = "MDSLW_COMPLETION")]
    pub completion: Option<Shell>,
    /// Specify the number of threads to use for processing files from disk in parallel. Defaults
    /// to the number of{n}   logical processors.
    #[arg(short, long, env = "MDSLW_JOBS")]
    pub jobs: Option<usize>,
    /// What to report to stdout, ignored when reading from stdin:
    /// {n}   * "none" => report nothing but be silent instead
    /// {n}   * "changed" => output the names of files that were changed
    /// {n}   * "state" => output <state>:<filename> where <state> is "U" for "unchanged" or
    ///       "C" for "changed"
    /// {n}   * "diff-myers" => output a unified diff based on the myers algorithm
    /// {n}   * "diff-patience" => output a unified diff based on the patience algorithm
    /// {n}   * "diff-lcs" => output a unified diff based on the lcs algorithm
    ///       {n}  .
    #[arg(value_enum, short, long, env = "MDSLW_REPORT", default_value_t = ReportMode::None)]
    pub report: ReportMode,
    /// Specify a downstream pager for diffs (with args) that reads diffs from stdin.
    /// {n}   Useful if you want to display a diff nicely. For example, specify
    /// {n}   "delta --side-by-side" to get a side-by-side view.
    #[arg(value_enum, short, long, env = "MDSLW_DIFF_PAGER")]
    pub diff_pager: Option<String>,
    /// The path to the file that is read from stdin. This is used to determine relevant config
    /// files{n}   when reading from stdin and to run an upstream formatter.
    #[arg(long, env = "MDSLW_STDIN_FILEPATH")]
    pub stdin_filepath: Option<PathBuf>,
    /// Output the default config file in TOML format to stdout and exit.
    #[arg(long, env = "MDSLW_DEFAULT_CONFIG")]
    pub default_config: bool,
    /// Specify to increase verbosity of log output. Specify multiple times to increase even
    /// further.
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

#[derive(Debug, PartialEq)]
pub struct PerFileCfg {
    pub max_width: usize,
    pub end_markers: String,
    pub lang: String,
    pub suppressions: String,
    pub ignores: String,
    pub upstream_command: String,
    pub upstream: String,
    pub upstream_separator: String,
    pub case: Case,
    pub link_actions: Option<LinkActions>,
    pub keep_whitespace: Option<KeepWhitespace>,
    pub format_block_quotes: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct CfgFile {
    pub max_width: Option<usize>,
    pub end_markers: Option<String>,
    pub lang: Option<String>,
    pub suppressions: Option<String>,
    pub ignores: Option<String>,
    pub upstream_command: Option<String>,
    pub upstream: Option<String>,
    pub upstream_separator: Option<String>,
    pub case: Option<Case>,
    pub link_actions: Option<LinkActions>,
    pub keep_whitespace: Option<KeepWhitespace>,
    pub format_block_quotes: Option<bool>,
}

impl CfgFile {
    /// Merge one config file into this one. Some-values in self take precedence. The return value
    /// indicates whether all fields of the struct are fully defined, which means that further
    /// merging won't have any effect.
    pub fn merge_with(&mut self, other: &Self) -> bool {
        let mut fully_defined = true;

        // Reduce code duplication with a macro.
        macro_rules! merge_field {
            ($field:ident) => {
                if self.$field.is_none() {
                    self.$field = other.$field.clone();
                }
                fully_defined = fully_defined && self.$field.is_some();
            };
        }

        merge_field!(max_width);
        merge_field!(end_markers);
        merge_field!(lang);
        merge_field!(suppressions);
        merge_field!(ignores);
        merge_field!(upstream_command);
        merge_field!(upstream);
        merge_field!(upstream_separator);
        merge_field!(case);
        merge_field!(link_actions);
        merge_field!(keep_whitespace);
        merge_field!(format_block_quotes);

        fully_defined
    }

    fn new() -> Self {
        Self {
            max_width: None,
            end_markers: None,
            lang: None,
            suppressions: None,
            ignores: None,
            upstream_command: None,
            upstream: None,
            upstream_separator: None,
            case: None,
            link_actions: None,
            keep_whitespace: None,
            format_block_quotes: None,
        }
    }
}

impl Default for CfgFile {
    fn default() -> Self {
        let no_args: Vec<OsStr> = vec![];
        let default_cli = CliArgs::parse_from(no_args);

        macro_rules! merge_fields {
            (@ | $($result:tt)*) => { 
                Self{ 
                    $($result)* 
                    link_actions: {
                        let val = default_cli.link_actions.resolve(None);
                        if val == LinkActions::None { None } else { Some(val) }
                    },
                    keep_whitespace: {
                        let val = default_cli.keep_whitespace.resolve(None);
                        if val == KeepWhitespace::None { None } else { Some(val) }
                    },
                } 
            };
            (@ $name:ident $($names:ident)* | $($result:tt)*) => {
                merge_fields!(
                    @ $($names)* |
                    $name: Some(default_cli.$name.resolve(None)),
                    $($result)*
                )
            };
            ($($names:ident)*) => { merge_fields!(@ $($names)* | ) };
        }

        merge_fields!(max_width end_markers lang suppressions ignores upstream_command upstream upstream_separator case format_block_quotes)
    }
}

pub fn merge_configs<'a, I>(cli: &CliArgs, files: I) -> PerFileCfg
where
    I: IntoIterator<Item = &'a (PathBuf, CfgFile)>,
{
    let mut merged = CfgFile::new();
    for (path, other) in files {
        log::debug!("merging config file {}", path.to_string_lossy());
        if merged.merge_with(other) {
            log::debug!("config fully defined, stopping merge");
            break;
        }
    }
    log::debug!("configuration loaded from files: {:?}", merged);
    log::debug!("configuration loaded from CLI: {:?}", cli);

    macro_rules! merge_fields {
        (@ | $($result:tt)*) => { 
            PerFileCfg{ 
                $($result)* 
                link_actions: {
                    let resolved = cli.link_actions.resolve(merged.link_actions);
                    if resolved == LinkActions::None { None } else { Some(resolved) }
                },
                keep_whitespace: {
                    let resolved = cli.keep_whitespace.resolve(merged.keep_whitespace);
                    if resolved == KeepWhitespace::None { None } else { Some(resolved) }
                },
            } 
        };
        (@ $name:ident $($names:ident)* | $($result:tt)*) => {
            merge_fields!(
                @ $($names)* |
                $name: cli.$name.resolve(merged.$name),
                $($result)*
            )
        };
        ($($names:ident)*) => { merge_fields!(@ $($names)* | ) };
    }

    let result = merge_fields!(max_width end_markers lang suppressions ignores upstream_command upstream upstream_separator case format_block_quotes);
    
    log::debug!("merged configuration: {:?}", result);
    result
}

#[cfg(test)]
mod test {
    use super::*;

    // Actual tests follow.
    #[test]
    fn merging_two_partially_defined_config_files() {
        let mut main_cfg = CfgFile {
            max_width: Some(10),
            end_markers: None,
            lang: None,
            suppressions: None,
            ignores: Some("some words".into()),
            upstream_command: None,
            upstream: None,
            upstream_separator: None,
            case: None,
            link_actions: None,
            keep_whitespace: None,
            format_block_quotes: None,
        };
        let other_cfg = CfgFile {
            max_width: None,
            end_markers: None,
            lang: Some("ac".into()),
            suppressions: None,
            ignores: None,
            upstream_command: Some("some".into()),
            upstream: None,
            upstream_separator: None,
            case: None,
            link_actions: Some(LinkActions::Both),
            keep_whitespace: None,
            format_block_quotes: None,
        };

        let fully_defined = main_cfg.merge_with(&other_cfg);
        assert!(!fully_defined);

        let expected_cfg = CfgFile {
            max_width: Some(10),
            end_markers: None,
            lang: Some("ac".into()),
            suppressions: None,
            ignores: Some("some words".into()),
            upstream_command: Some("some".into()),
            upstream: None,
            upstream_separator: None,
            case: None,
            link_actions: Some(LinkActions::Both),
            keep_whitespace: None,
            format_block_quotes: None,
        };

        assert_eq!(expected_cfg, main_cfg);
    }

    #[test]
    fn options_in_main_config_are_kept() {
        let mut main_cfg = CfgFile {
            max_width: Some(10),
            end_markers: None,
            lang: None,
            suppressions: None,
            ignores: Some("some words".into()),
            upstream_command: None,
            upstream: None,
            upstream_separator: None,
            case: None,
            link_actions: None,
            keep_whitespace: None,
            format_block_quotes: None,
        };
        let other_cfg = CfgFile {
            max_width: Some(20),
            end_markers: None,
            lang: None,
            suppressions: None,
            ignores: Some("some other words".into()),
            upstream_command: None,
            upstream: None,
            upstream_separator: None,
            case: None,
            link_actions: None,
            keep_whitespace: None,
            format_block_quotes: None,
        };
        assert_ne!(main_cfg, other_cfg);

        let fully_defined = main_cfg.merge_with(&other_cfg);
        assert!(!fully_defined);

        let expected_cfg = CfgFile {
            max_width: Some(10),
            end_markers: None,
            lang: None,
            suppressions: None,
            ignores: Some("some words".into()),
            upstream_command: None,
            upstream: None,
            upstream_separator: None,
            case: None,
            link_actions: None,
            keep_whitespace: None,
            format_block_quotes: None,
        };

        assert_eq!(expected_cfg, main_cfg);
    }

    #[test]
    fn fully_defined_config_is_immutable() {
        let mut main_cfg = CfgFile {
            max_width: None,
            end_markers: None,
            lang: None,
            suppressions: None,
            ignores: None,
            upstream_command: None,
            upstream: None,
            upstream_separator: None,
            case: None,
            link_actions: None,
            keep_whitespace: None,
            format_block_quotes: None,
        };
        let missing_options = CfgFile {
            max_width: Some(20),
            end_markers: Some("marker".into()),
            lang: Some("lang".into()),
            suppressions: Some("suppressions".into()),
            ignores: Some("some other words".into()),
            upstream_command: Some("upstream-command".into()),
            upstream: Some("upstream".into()),
            upstream_separator: Some("sep".into()),
            case: Some(Case::Ignore),
            link_actions: Some(LinkActions::Both),
            keep_whitespace: Some(KeepWhitespace::Both),
            format_block_quotes: Some(true),
        };
        let other_options = CfgFile {
            max_width: Some(10),
            end_markers: Some("nothing".into()),
            lang: Some("asdf".into()),
            suppressions: Some("just text".into()),
            ignores: Some("ignore this".into()),
            upstream_command: Some("does not matter".into()),
            upstream: Some("swimming is nice".into()),
            upstream_separator: Some("let's not split up".into()),
            case: Some(Case::Keep),
            link_actions: Some(LinkActions::CollateDefs),
            keep_whitespace: Some(KeepWhitespace::InLinks),
            format_block_quotes: Some(false),
        };

        let fully_defined = main_cfg.merge_with(&missing_options);
        assert!(fully_defined);
        let fully_defined = main_cfg.merge_with(&other_options);
        assert!(fully_defined);

        let expected_cfg = CfgFile {
            max_width: Some(20),
            end_markers: Some("marker".into()),
            lang: Some("lang".into()),
            suppressions: Some("suppressions".into()),
            ignores: Some("some other words".into()),
            upstream_command: Some("upstream-command".into()),
            upstream: Some("upstream".into()),
            upstream_separator: Some("sep".into()),
            case: Some(Case::Ignore),
            link_actions: Some(LinkActions::Both),
            keep_whitespace: Some(KeepWhitespace::Both),
            format_block_quotes: Some(true),
        };

        assert_eq!(expected_cfg, main_cfg);
    }

    #[test]
    fn merging_cli_with_two_config_files() {
        let main_cfg = CfgFile {
            max_width: Some(10),
            end_markers: None,
            lang: None,
            suppressions: None,
            ignores: Some("some words".into()),
            upstream_command: None,
            upstream: None,
            upstream_separator: None,
            case: None,
            link_actions: None,
            keep_whitespace: None,
            format_block_quotes: None,
        };
        let other_cfg = CfgFile {
            max_width: None,
            end_markers: None,
            lang: Some("ac".into()),
            suppressions: None,
            ignores: None,
            upstream_command: None,
            upstream: None,
            upstream_separator: None,
            case: None,
            link_actions: Some(LinkActions::Both),
            keep_whitespace: None,
            format_block_quotes: None,
        };
        let default_cfg = CfgFile::default();

        let files = vec![
            (PathBuf::from("main"), main_cfg),
            (PathBuf::from("other"), other_cfg),
            (PathBuf::from("default"), default_cfg),
        ];
        let no_args: Vec<OsStr> = vec![];
        let cli = CliArgs::parse_from(no_args);
        let merged = merge_configs(&cli, &files);

        let expected_cfg = PerFileCfg {
            max_width: 10,
            end_markers: "?!:.".into(),
            lang: "ac".into(),
            suppressions: "".into(),
            ignores: "some words".into(),
            upstream_command: "".into(),
            upstream: "".into(),
            upstream_separator: "".into(),
            case: Case::Ignore,
            link_actions: Some(LinkActions::Both),
            keep_whitespace: None,
            format_block_quotes: false,
        };

        assert_eq!(expected_cfg, merged);
    }
}
