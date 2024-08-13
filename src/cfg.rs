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

use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use clap_complete::Shell;

// Command-line interface definition.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum OpMode {
    Both,
    Check,
    Format,
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum Case {
    Ignore,
    Keep,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum ReportMode {
    None,
    Changed,
    State,
    DiffMeyers,
    DiffPatience,
    DiffLcs,
}

impl ReportMode {
    pub fn is_diff_mode(&self) -> bool {
        self == &ReportMode::DiffMeyers
            || self == &ReportMode::DiffPatience
            || self == &ReportMode::DiffLcs
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    /// Paths to files or directories that shall be processed.
    pub paths: Vec<PathBuf>,
    /// The maximum line width that is acceptable. A value of 0 disables wrapping of{n}   long
    /// lines.
    #[arg(short = 'w', long, env = "MDSLW_MAX_WIDTH", default_value_t = 80)]
    pub max_width: usize,
    /// A set of characters that are acceptable end of sentence markers.
    #[arg(short, long, env = "MDSLW_END_MARKERS", default_value_t = String::from("?!:."))]
    pub end_markers: String,
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
    #[arg(short, long, env = "MDSLW_LANG", default_value_t = String::from("ac"))]
    pub lang: String,
    /// Space-separated list of words that end in one of END_MARKERS but that should not be
    /// followed by a line{n}   break. This is in addition to what is specified via --lang.
    #[arg(short, long, env = "MDSLW_SUPPRESSIONS", default_value_t = String::from(""))]
    pub suppressions: String,
    /// Space-separated list of words that end in one of END_MARKERS and that should be
    /// removed{n}   from the list of suppressions.
    #[arg(short, long, env = "MDSLW_IGNORES", default_value_t = String::from(""))]
    pub ignores: String,
    /// Specify an upstream auto-formatter (with args) that reads from stdin and writes to stdout.
    /// {n}   It will be called before mdslw will run. Useful if you want to chain multiple
    /// tools.{n}   For example, specify "prettier --parser=markdown" to call prettier first.
    /// Run{n}   in each file's directory if PATHS are specified.
    #[arg(short, long, env = "MDSLW_UPSTREAM", default_value_t = String::new())]
    pub upstream: String,
    /// How to handle the case of provided suppression words, both via --lang
    /// and{n}   --suppressions
    #[arg(value_enum, short, long, env = "MDSLW_CASE", default_value_t = Case::Ignore)]
    pub case: Case,
    /// The file extension used to find markdown files when an entry in{n}   PATHS is a directory.
    #[arg(long, env = "MDSLW_EXTENSION", default_value_t = String::from(".md"))]
    pub extension: String,
    // The "." below is used to cause clap to format the help message nicely.
    /// Comma-separated list of optional features to enable or disable. Currently, the following
    /// are supported:
    /// {n}   * keep-spaces-in-links => do not replace spaces in link texts by non-breaking spaces
    /// {n}   * keep-linebreaks => do not remove existing linebreaks during the line-wrapping
    ///         process
    /// {n}  .
    #[arg(long, env = "MDSLW_FEATURES", default_value_t = String::new())]
    pub features: String,
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
    #[arg(value_enum, short, long, env = "MDSLW_REPORT")]
    pub diff_pager: Option<String>,
    /// Specify to increase verbosity of log output. Specify multiple times to increase even
    /// further.
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

impl CliArgs {
    pub fn to_per_file_cfg(&self) -> PerFileCfg {
        PerFileCfg {
            max_width: self.max_width,
            end_markers: self.end_markers.clone(),
            lang: self.lang.clone(),
            suppressions: self.suppressions.clone(),
            ignores: self.ignores.clone(),
            upstream: self.upstream.clone(),
            case: self.case,
            features: self.features.clone(),
        }
    }
}

pub struct PerFileCfg {
    pub max_width: usize,
    pub end_markers: String,
    pub lang: String,
    pub suppressions: String,
    pub ignores: String,
    pub upstream: String,
    pub case: Case,
    pub features: String,
}

#[cfg(test)]
mod test {
    use super::*;

    // Code kept here for posterity but in order to keep it out of the production code base.

    // use serde::Deserialize;
    #[derive(Debug, PartialEq, Eq)]
    pub struct CfgFile {
        pub max_width: Option<usize>,
        pub end_markers: Option<String>,
        pub lang: Option<String>,
        pub suppressions: Option<String>,
        pub ignores: Option<String>,
        pub upstream: Option<String>,
        pub case: Option<Case>,
        pub features: Option<String>,
    }

    impl CfgFile {
        /// Merge one config file into this one. Some-values in self take precedence. The return value
        /// indicates whether all fields of the struct are fully defined, which means that further
        /// merging won't have any effect.
        pub fn merge_with(&mut self, other: Self) -> bool {
            let mut fully_defined = true;

            // Reduce code duplication with a macro.
            macro_rules! merge_field {
                ($field:ident) => {
                    if self.$field.is_none() {
                        self.$field = other.$field;
                    }
                    fully_defined = fully_defined && self.$field.is_some();
                };
            }

            merge_field!(max_width);
            merge_field!(end_markers);
            merge_field!(lang);
            merge_field!(suppressions);
            merge_field!(ignores);
            merge_field!(upstream);
            merge_field!(case);
            merge_field!(features);

            fully_defined
        }
    }

    // Actual tests follow.
    #[test]
    fn merging_two_partially_defined_config_files() {
        let mut main_cfg = CfgFile {
            max_width: Some(10),
            end_markers: None,
            lang: None,
            suppressions: None,
            ignores: Some("some words".into()),
            upstream: None,
            case: None,
            features: None,
        };
        let other_cfg = CfgFile {
            max_width: None,
            end_markers: None,
            lang: Some("ac".into()),
            suppressions: None,
            ignores: None,
            upstream: None,
            case: None,
            features: Some("feature".into()),
        };

        let fully_defined = main_cfg.merge_with(other_cfg);
        assert!(!fully_defined);

        let expected_cfg = CfgFile {
            max_width: Some(10),
            end_markers: None,
            lang: Some("ac".into()),
            suppressions: None,
            ignores: Some("some words".into()),
            upstream: None,
            case: None,
            features: Some("feature".into()),
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
            upstream: None,
            case: None,
            features: None,
        };
        let other_cfg = CfgFile {
            max_width: Some(20),
            end_markers: None,
            lang: None,
            suppressions: None,
            ignores: Some("some other words".into()),
            upstream: None,
            case: None,
            features: None,
        };
        assert_ne!(main_cfg, other_cfg);

        let fully_defined = main_cfg.merge_with(other_cfg);
        assert!(!fully_defined);

        let expected_cfg = CfgFile {
            max_width: Some(10),
            end_markers: None,
            lang: None,
            suppressions: None,
            ignores: Some("some words".into()),
            upstream: None,
            case: None,
            features: None,
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
            upstream: None,
            case: None,
            features: None,
        };
        let missing_options = CfgFile {
            max_width: Some(20),
            end_markers: Some("marker".into()),
            lang: Some("lang".into()),
            suppressions: Some("suppressions".into()),
            ignores: Some("some other words".into()),
            upstream: Some("upstream".into()),
            case: Some(Case::Ignore),
            features: Some("feature".into()),
        };
        let other_options = CfgFile {
            max_width: Some(10),
            end_markers: Some("nothing".into()),
            lang: Some("asdf".into()),
            suppressions: Some("just text".into()),
            ignores: Some("ignore this".into()),
            upstream: Some("swimming is nice".into()),
            case: Some(Case::Keep),
            features: Some("everything".into()),
        };

        let fully_defined = main_cfg.merge_with(missing_options);
        assert!(fully_defined);
        let fully_defined = main_cfg.merge_with(other_options);
        assert!(fully_defined);

        let expected_cfg = CfgFile {
            max_width: Some(20),
            end_markers: Some("marker".into()),
            lang: Some("lang".into()),
            suppressions: Some("suppressions".into()),
            ignores: Some("some other words".into()),
            upstream: Some("upstream".into()),
            case: Some(Case::Ignore),
            features: Some("feature".into()),
        };

        assert_eq!(expected_cfg, main_cfg);
    }
}
