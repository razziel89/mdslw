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

mod call;
mod detect;
mod features;
mod fs;
mod indent;
mod lang;
mod linebreak;
mod parse;
mod ranges;
mod replace;
mod wrap;

use std::path::PathBuf;

use anyhow::{Context, Error, Result};
use clap::{Parser, ValueEnum};

use crate::call::upstream_formatter;
use crate::detect::BreakDetector;
use crate::features::FeatureCfg;
use crate::fs::find_files_with_extension;
use crate::lang::keep_word_list;
use crate::parse::parse_markdown;
use crate::ranges::fill_markdown_ranges;
use crate::replace::replace_spaces_in_links_by_nbsp;
use crate::wrap::add_linebreaks_and_wrap;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OpMode {
    Both,
    Check,
    Format,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Case {
    Ignore,
    Keep,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Paths to files or directories that shall be processed.
    paths: Vec<PathBuf>,
    /// The maximum line width that is acceptable. A value of 0 disables wrapping of{n}   long
    /// lines.
    #[arg(short = 'w', long, env = "MDSLW_MAX_WIDTH", default_value_t = 80)]
    max_width: usize,
    /// A set of characters that are acceptable end of sentence markers.
    #[arg(short, long, env = "MDSLW_END_MARKERS", default_value_t = String::from("?!:."))]
    end_markers: String,
    /// Mode of operation: "check" means exit with error if format has to be adjusted but do not
    /// format,{n}   "format" means format the file and exit with error in case of problems only,
    /// "both" means do both{n}   (useful as pre-commit hook).
    #[arg(value_enum, short, long, env = "MDSLW_MODE", default_value_t = OpMode::Format)]
    mode: OpMode,
    /// A space-separated list of languages whose suppression words as specified by unicode should
    /// be {n}   taken into account. See here for all languages:
    /// {n}   https://github.com/unicode-org/cldr-json/tree/main/cldr-json/cldr-segments-full/segments
    /// {n}   Use "none" to disable.
    /// Supported languages are: de en es fr it. Use "ac" for "author's choice",{n}   a list
    /// for the Enlish language defined by this tool's author.
    #[arg(short, long, env = "MDSLW_LANG", default_value_t = String::from("ac"))]
    lang: String,
    /// Space-separated list of words that end in one of END_MARKERS but that should not be
    /// followed by a line{n}   break. This is in addition to what is specified via --lang.
    #[arg(short, long, env = "MDSLW_SUPPRESSIONS", default_value_t = String::from(""))]
    suppressions: String,
    /// Space-separated list of words that end in one of END_MARKERS and that should be
    /// removed{n}   from the list of suppressions.
    #[arg(short, long, env = "MDSLW_IGNORES", default_value_t = String::from(""))]
    ignores: String,
    /// Specify an upstream auto-formatter (with args) that reads from stdin and writes to stdout.
    /// {n}   It will be called before mdslw will run. Useful if you want to chain multiple
    /// tools.{n}   For example, specify "prettier --parser=markdown" to call prettier first.
    /// Run{n}   in each file's directory if PATHS are specified.
    #[arg(short, long, env = "MDSLW_UPSTREAM")]
    upstream: Option<String>,
    /// How to handle the case of provided suppression words, both via --lang
    /// and{n}   --suppressions
    #[arg(value_enum, short, long, env = "MDSLW_CASE", default_value_t = Case::Ignore)]
    case: Case,
    /// The file extension used to find markdown files when an entry in{n}   PATHS is a directory.
    #[arg(long, env = "MDSLW_EXTENSION", default_value_t = String::from(".md"))]
    extension: String,
    // The "." below is used to cause clap to format the help message nicely.
    /// Comma-separated list of optional features to enable or disable. Currently, the following
    /// are supported:
    /// {n}   * keep-spaces-in-links => do not replace spaces in link texts by non-breaking spaces
    /// {n}   * keep-inline-html => prevent modifications of HTML that does not span lines
    /// {n}   * keep-footnotes => prevent modifications to footnotes
    /// {n}   * modify-tasklists => allow modifications to tasklists
    /// {n}   * modify-tables => allow modifications to tables (entire tables, not inside tables)
    /// {n}   * modify-nbsp => allow modifications to UTF8 non-breaking spaces
    /// {n}   * breaking-multiple-markers => insert line breaks after repeated end markers
    /// {n}   * breaking-start-marker => insert line breaks after a single end marker at the
    ///         beginning of a line
    /// {n}  .
    #[arg(long, env = "MDSLW_FEATURES", default_value_t = String::new())]
    features: String,
}

fn read_stdin() -> String {
    std::io::stdin()
        .lines()
        // Interrupt as soon as one line could not be read.
        .map_while(Result::ok)
        .collect::<Vec<_>>()
        .join("\n")
}

fn get_cwd() -> Result<PathBuf> {
    std::env::current_dir()
        .context("failed to get current working directory")
        .map(|el| el.as_path().to_owned())
        .and_then(|el| std::fs::canonicalize(el).context("failed to canonicalise path"))
}

fn process(
    text: String,
    file_dir: PathBuf,
    upstream: &Option<String>,
    max_width: &Option<usize>,
    detector: &BreakDetector,
    feature_cfg: &FeatureCfg,
) -> Result<String> {
    // Keep newlines at the end of the file in tact. They disappear sometimes.
    let last_char = if text.ends_with('\n') { "\n" } else { "" };

    let after_upstream = if let Some(upstream) = upstream {
        upstream_formatter(upstream, text, file_dir)?
    } else {
        text
    };

    let after_map = if feature_cfg.keep_spaces_in_links {
        after_upstream
    } else {
        replace_spaces_in_links_by_nbsp(after_upstream)
    };

    let parsed = parse_markdown(&after_map, &feature_cfg.parse_cfg);
    let filled = fill_markdown_ranges(parsed, &after_map);
    let formatted = add_linebreaks_and_wrap(filled, max_width, detector, &after_map);

    let file_end = if !formatted.ends_with(last_char) {
        last_char
    } else {
        ""
    };

    Ok(format!("{}{}", formatted, file_end))
}

pub fn get_file_content_and_dir(path: &PathBuf) -> Result<(String, PathBuf)> {
    let text = std::fs::read_to_string(path).context("failed to read file")?;
    let dir = path
        .parent()
        .ok_or(Error::msg("failed to determine parent directory"))?
        .to_path_buf();

    Ok((text, dir))
}

fn main() -> Result<()> {
    let cli = Args::parse();

    let lang_keep_words = keep_word_list(&cli.lang).context("loading keep words for languages")?;

    let feature_cfg = cli
        .features
        .parse::<FeatureCfg>()
        .context("parsing selected features")?;

    let detector = BreakDetector::new(
        &(lang_keep_words + &cli.suppressions),
        &cli.ignores,
        cli.case == Case::Keep,
        cli.end_markers,
        &feature_cfg.break_cfg,
    );

    let max_width = if cli.max_width == 0 {
        None
    } else {
        Some(cli.max_width)
    };

    let unchanged = if cli.paths.is_empty() {
        // Process content from stdin and write to stdout.
        let text = read_stdin();
        let cwd = get_cwd()?;

        let processed = process(
            text.clone(),
            cwd,
            &cli.upstream,
            &max_width,
            &detector,
            &feature_cfg,
        )?;

        // Decide what to output.
        match cli.mode {
            OpMode::Format | OpMode::Both => {
                println!("{}", processed);
            }
            OpMode::Check => {
                // In check mode, we output the original content when reading from stdin.
                println!("{}", text);
            }
        }

        processed == text
    } else {
        let md_files = find_files_with_extension(cli.paths, &cli.extension)
            .context("failed to discover markdown files")?;

        let cwd_name = get_cwd()?.to_string_lossy().to_string();
        // Process all MD files we found and abort on any error. We will update files in-place.
        let mut has_changed = false;

        let change_str = match cli.mode {
            OpMode::Format | OpMode::Both => "CHANGED",
            OpMode::Check => "WOULD BE CHANGED",
        };

        for path in md_files {
            let abspath = path.to_string_lossy();
            let relpath = abspath
                .strip_prefix(&cwd_name)
                .map(|el| el.trim_start_matches(std::path::MAIN_SEPARATOR))
                .unwrap_or(&abspath);
            let context = || format!("failed to process file: {}", relpath);

            let (text, file_dir) = get_file_content_and_dir(&path).with_context(context)?;

            let processed = process(
                text.clone(),
                file_dir,
                &cli.upstream,
                &max_width,
                &detector,
                &feature_cfg,
            )
            .with_context(context)?;

            // Decide whether to overwrite existing files.
            match cli.mode {
                OpMode::Format | OpMode::Both => {
                    std::fs::write(&path, processed.as_bytes()).with_context(context)?;
                }
                // Do not write anything in check mode.
                OpMode::Check => {}
            }

            if processed == text {
                eprintln!("{} -> OK", relpath);
            } else {
                eprintln!("{} -> {}", relpath, change_str);
                has_changed = true;
            }
        }

        !has_changed
    };

    // Process exit code.
    match cli.mode {
        OpMode::Format => Ok(()),
        OpMode::Check | OpMode::Both => {
            if unchanged {
                Ok(())
            } else {
                Err(Error::msg("at least one processed file changed"))
            }
        }
    }
}
