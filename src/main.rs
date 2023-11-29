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
mod ignore;
mod indent;
mod lang;
mod linebreak;
mod logging;
mod parse;
mod ranges;
mod replace;
mod wrap;

use std::io;
use std::path::PathBuf;

use anyhow::{Context, Error, Result};
use clap::{CommandFactory, Parser, ValueEnum};
use clap_complete::{generate, Shell};
use rayon::prelude::*;

use crate::call::upstream_formatter;
use crate::detect::BreakDetector;
use crate::features::FeatureCfg;
use crate::fs::find_files_with_extension;
use crate::lang::keep_word_list;
use crate::logging::Logger;
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
struct CliArgs {
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
    /// Output shell completion file for the given shell to stdout and exit.{n}  .
    #[arg(value_enum, long, env = "MDSLW_COMPLETION")]
    completion: Option<Shell>,
    /// Specify the number of threads to use for processing files from disk in parallel. Defaults
    /// to the number of{n}   logical processors.
    #[arg(short, long, env = "MDSLW_JOBS")]
    jobs: Option<usize>,
    /// Specify to increase verbosity of log output. Specify multiple times to increase even
    /// further.
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
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
) -> Result<(String, bool)> {
    let after_upstream = if let Some(upstream) = upstream {
        log::debug!("calling upstream formatter: {}", upstream);
        upstream_formatter(upstream, text.clone(), file_dir)?
    } else {
        log::debug!("not calling any upstream formatter");
        text.clone()
    };

    let after_map = if feature_cfg.keep_spaces_in_links {
        log::debug!("not replacing spaces in links by non-breaking spaces");
        after_upstream
    } else {
        log::debug!("replacing spaces in links by non-breaking spaces");
        replace_spaces_in_links_by_nbsp(after_upstream)
    };

    let parsed = parse_markdown(&after_map, &feature_cfg.parse_cfg);
    let filled = fill_markdown_ranges(parsed, &after_map);
    let formatted = add_linebreaks_and_wrap(filled, max_width, detector, &after_map);

    // Keep newlines at the end of the file in tact. They disappear sometimes.
    let file_end = if !formatted.ends_with('\n') && text.ends_with('\n') {
        log::debug!("adding missing trailing newline character");
        "\n"
    } else {
        ""
    };

    let processed = format!("{}{}", formatted, file_end);
    let unchanged = processed == text;

    Ok((processed, unchanged))
}

pub fn get_file_content_and_dir(path: &PathBuf) -> Result<(String, PathBuf)> {
    let text = std::fs::read_to_string(path).context("failed to read file")?;
    let dir = path
        .parent()
        .map(|el| el.to_path_buf())
        .ok_or(Error::msg("failed to determine parent directory"))?;

    Ok((text, dir))
}

fn init_logging(level: u8) -> Result<(), log::SetLoggerError> {
    log::set_boxed_logger(Box::new(Logger::new(level)))
        .map(|()| log::set_max_level(log::LevelFilter::Trace))
}

fn process_stdin<TextFn>(mode: &OpMode, process_text: TextFn) -> Result<bool>
where
    TextFn: Fn(String, PathBuf) -> Result<(String, bool)>,
{
    log::debug!("processing content from stdin and writing to stdout");
    let text = read_stdin();
    let cwd = get_cwd()?;

    let (processed, unchanged) = process_text(text.clone(), cwd)?;

    // Decide what to output.
    match mode {
        OpMode::Format | OpMode::Both => {
            log::debug!("writing modified file to stdout");
            println!("{}", processed);
        }
        OpMode::Check => {
            log::debug!("writing original file to stdout in check mode");
            println!("{}", text);
        }
    }

    Ok(unchanged)
}

fn process_file<TextFn>(mode: &OpMode, path: &PathBuf, process_text: TextFn) -> Result<bool>
where
    TextFn: Fn(String, PathBuf) -> Result<(String, bool)>,
{
    let cwd_name = get_cwd()?.to_string_lossy().to_string();
    let abspath = path.to_string_lossy();
    // Report the relative path if the file is located further down the directory tree. Otherwise,
    // report the absolute path. Also report the absolute path in case there are some weird
    // characters in the path that prevent conversion to UTF8.
    let report_path = abspath
        .strip_prefix(&cwd_name)
        .map(|el| el.trim_start_matches(std::path::MAIN_SEPARATOR))
        .unwrap_or(&abspath);
    log::debug!("processing {}", report_path);

    let (text, file_dir) = get_file_content_and_dir(path)?;
    let (processed, unchanged) = process_text(text, file_dir)?;

    if unchanged {
        log::info!("{} -> OK", report_path);
    } else {
        // Decide whether to overwrite existing files.
        match mode {
            OpMode::Format | OpMode::Both => {
                log::debug!("modifying file {} in place", path.to_string_lossy());
                std::fs::write(path, processed.as_bytes()).context("failed to write file")?;
                log::info!("{} -> CHANGED", report_path);
            }
            // Do not write anything in check mode.
            OpMode::Check => {
                log::debug!("not modifying file {}", path.to_string_lossy());
                log::info!("{} -> WOULD BE CHANGED", report_path);
            }
        }
    }

    Ok(unchanged)
}

fn main() -> Result<()> {
    let cli = CliArgs::parse();

    // Initialise logging as early as possible.
    init_logging(cli.verbose)?;

    if let Some(shell) = cli.completion {
        log::info!("generating shell completion for {}", shell);
        let mut cmd = CliArgs::command();
        let name = cmd.get_name().to_string();
        generate(shell, &mut cmd, name, &mut io::stdout());
        return Ok(());
    }

    let lang_keep_words = keep_word_list(&cli.lang).context("cannot load keep words")?;

    let feature_cfg = cli
        .features
        .parse::<FeatureCfg>()
        .context("cannot parse selected features")?;

    let detector = BreakDetector::new(
        &(lang_keep_words + &cli.suppressions),
        &cli.ignores,
        cli.case == Case::Keep,
        cli.end_markers,
        &feature_cfg.break_cfg,
    );

    let max_width = if cli.max_width == 0 {
        log::debug!("not limiting line length");
        None
    } else {
        log::debug!("limiting line length to {} characters", cli.max_width);
        Some(cli.max_width)
    };

    let process_text = move |text, file_dir| {
        process(
            text,
            file_dir,
            &cli.upstream,
            &max_width,
            &detector,
            &feature_cfg,
        )
    };

    let (unchanged, process_result_exit_code) = if cli.paths.is_empty() {
        match process_stdin(&cli.mode, process_text) {
            Ok(unchanged) => (unchanged, Ok(())),
            Err(err) => (true, Err(err)),
        }
    } else {
        let md_files = find_files_with_extension(cli.paths, &cli.extension)
            .context("failed to discover markdown files")?;
        log::debug!("will process {} file(s) from disk", md_files.len());

        // Set number of threads depending on user's choice.
        if let Some(num_jobs) = cli.jobs {
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_jobs)
                .build_global()
                .context("failed to initialise processing thread-pool")?;
        }

        // Process all MD files we found.
        let (no_file_changed, has_error) = md_files
            .par_iter()
            .map(|path| match process_file(&cli.mode, path, &process_text) {
                Ok(unchanged) => (unchanged, false),
                Err(err) => {
                    log::error!("failed to process {}: {:?}", path.to_string_lossy(), err);
                    (true, true)
                }
            })
            // First element is true if document was unchanged. Second element is true if there had
            // been an error.
            .reduce(|| (true, false), |a, b| (a.0 && b.0, a.1 || b.1));

        let default_exit_code = if has_error {
            Err(Error::msg("there were errors processing at least one file"))
        } else {
            Ok(())
        };
        (no_file_changed, default_exit_code)
    };

    log::debug!("finished execution");
    // Process exit code.
    if unchanged {
        process_result_exit_code
    } else {
        match cli.mode {
            OpMode::Format => process_result_exit_code,
            OpMode::Check => Err(Error::msg("at least one processed file would be changed")),
            OpMode::Both => Err(Error::msg("at least one processed file changed")),
        }
    }
}
