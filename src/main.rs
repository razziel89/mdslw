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

// Imports.
mod call;
mod cfg;
mod detect;
mod diff;
mod features;
mod frontmatter;
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
use std::path::{Path, PathBuf};

use anyhow::{Context, Error, Result};
use clap::{CommandFactory, Parser};
use clap_complete::generate;
use rayon::prelude::*;

fn generate_report(
    mode: &cfg::ReportMode,
    new: &str,
    org: &str,
    filename: &Path,
) -> Option<String> {
    match mode {
        cfg::ReportMode::None => None,
        cfg::ReportMode::Changed => {
            if new != org {
                Some(format!("{}", filename.to_string_lossy()))
            } else {
                None
            }
        }
        cfg::ReportMode::State => {
            let ch = if new == org { 'U' } else { 'C' };
            Some(format!("{}:{}", ch, filename.to_string_lossy()))
        }
        cfg::ReportMode::DiffMeyers => Some(diff::Algo::Myers.generate(new, org, filename)),
        cfg::ReportMode::DiffPatience => Some(diff::Algo::Patience.generate(new, org, filename)),
        cfg::ReportMode::DiffLcs => Some(diff::Algo::Lcs.generate(new, org, filename)),
    }
}

fn process(document: String, file_dir: PathBuf, cfg: &cfg::PerFileCfg) -> Result<(String, String)> {
    // Prepare user-configured options. These could be outsourced if we didn't
    // allow per-file configurations.
    let lang_keep_words = lang::keep_word_list(&cfg.lang).context("cannot load keep words")?;
    let feature_cfg = cfg
        .features
        .parse::<features::FeatureCfg>()
        .context("cannot parse selected features")?;
    let detector = detect::BreakDetector::new(
        &(lang_keep_words + &cfg.suppressions),
        &cfg.ignores,
        cfg.case == cfg::Case::Keep,
        &cfg.end_markers,
        &feature_cfg.break_cfg,
    );
    let max_width = if cfg.max_width == 0 {
        log::debug!("not limiting line length");
        None
    } else {
        log::debug!("limiting line length to {} characters", cfg.max_width);
        Some(cfg.max_width)
    };

    // Actually process the text.
    let (frontmatter, text) = frontmatter::split_frontmatter(document.clone());
    // TODO: take config from frontmatter, if present.

    let after_upstream = if !cfg.upstream.is_empty() {
        log::debug!("calling upstream formatter: {}", cfg.upstream);
        call::upstream_formatter(&cfg.upstream, text, file_dir)?
    } else {
        log::debug!("not calling any upstream formatter");
        text
    };

    let after_space_replace = if feature_cfg.keep_spaces_in_links {
        log::debug!("not replacing spaces in links by non-breaking spaces");
        after_upstream
    } else {
        log::debug!("replacing spaces in links by non-breaking spaces");
        replace::replace_spaces_in_links_by_nbsp(after_upstream)
    };

    let parsed = parse::parse_markdown(&after_space_replace, &feature_cfg.parse_cfg);
    let filled = ranges::fill_markdown_ranges(parsed, &after_space_replace);
    let formatted =
        wrap::add_linebreaks_and_wrap(filled, &max_width, &detector, &after_space_replace);

    // Keep newlines at the end of the file in tact. They disappear sometimes.
    let file_end = if !formatted.ends_with('\n') && document.ends_with('\n') {
        log::debug!("adding missing trailing newline character");
        "\n"
    } else {
        ""
    };

    let processed = format!("{}{}{}", frontmatter, formatted, file_end);
    Ok((processed, document))
}

fn process_stdin(mode: &cfg::OpMode, cfg: &cfg::PerFileCfg) -> Result<bool> {
    log::debug!("processing content from stdin and writing to stdout");
    let text = fs::read_stdin();

    // The path "." means "current directory".
    let (processed, text) = process(text, PathBuf::from("."), cfg)?;

    // Decide what to output.
    match mode {
        cfg::OpMode::Format | cfg::OpMode::Both => {
            log::debug!("writing modified file to stdout");
            println!("{}", processed);
        }
        cfg::OpMode::Check => {
            log::debug!("writing original file to stdout in check mode");
            println!("{}", text);
        }
    }

    Ok(processed == text)
}

fn process_file(
    mode: &cfg::OpMode,
    path: &PathBuf,
    cfg: &cfg::PerFileCfg,
) -> Result<(String, String)> {
    let report_path = path.to_string_lossy();
    log::debug!("processing {}", report_path);

    let (text, file_dir) = fs::get_file_content_and_dir(path)?;
    let (processed, text) = process(text, file_dir, cfg)?;

    // Decide whether to overwrite existing files.
    match mode {
        cfg::OpMode::Format | cfg::OpMode::Both => {
            log::debug!("modifying file {} in place", report_path);
            std::fs::write(path, processed.as_bytes()).context("failed to write file")?;
            log::info!("{} -> CHANGED", path.to_string_lossy());
        }
        // Do not write anything in check mode.
        cfg::OpMode::Check => {
            log::debug!("not modifying file {}", report_path);
            log::info!("{} -> WOULD BE CHANGED", report_path);
        }
    }

    Ok((processed, text))
}

fn main() -> Result<()> {
    // Perform actions that cannot be changed on a per-file level.
    // Argument parsing.
    let cli = cfg::CliArgs::parse();
    // Initialising logging.
    logging::init_logging(cli.verbose)?;
    // Generation of shell completion.
    if let Some(shell) = cli.completion {
        log::info!("generating shell completion for {}", shell);
        let mut cmd = cfg::CliArgs::command();
        let name = cmd.get_name().to_string();
        generate(shell, &mut cmd, name, &mut io::stdout());
        return Ok(());
    }

    // All other actions can be specified on a per-file level.
    let unchanged = if cli.paths.is_empty() {
        process_stdin(&cli.mode, &cli.to_per_file_cfg())
    } else {
        let md_files = fs::find_files_with_extension(&cli.paths, &cli.extension)
            .context("failed to discover markdown files")?;
        log::debug!("will process {} file(s) from disk", md_files.len());

        // Set number of threads depending on user's choice.
        if let Some(num_jobs) = cli.jobs {
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_jobs)
                .build_global()
                .context("failed to initialise processing thread-pool")?;
        }

        // Enable pager only for diff output.
        let diff_pager = if cli.report.is_diff_mode() {
            &cli.diff_pager
        } else {
            log::info!("disabling possibly set diff pager for non-diff report");
            &None
        };
        let par_printer = call::ParallelPrinter::new(diff_pager)?;

        // Process all MD files we found.
        md_files
            .par_iter()
            .map(
                |path| match process_file(&cli.mode, path, &cli.to_per_file_cfg()) {
                    Ok((processed, text)) => {
                        if let Some(rep) = generate_report(&cli.report, &processed, &text, path) {
                            par_printer.println(&rep);
                        }
                        Ok(processed == text)
                    }
                    Err(err) => {
                        log::error!("failed to process {}: {:?}", path.to_string_lossy(), err);
                        Err(Error::msg("there were errors processing at least one file"))
                    }
                },
            )
            .reduce(
                || Ok(true),
                |a, b| match (a, b) {
                    (Err(err), _) => Err(err),
                    (_, Err(err)) => Err(err),
                    (Ok(f1), Ok(f2)) => Ok(f1 && f2),
                },
            )
    };

    log::debug!("finished execution");
    // Process exit code.
    match unchanged {
        Ok(true) => Ok(()),
        Ok(false) => match cli.mode {
            cfg::OpMode::Format => Ok(()),
            cfg::OpMode::Check => Err(Error::msg("at least one processed file would be changed")),
            cfg::OpMode::Both => Err(Error::msg("at least one processed file changed")),
        },
        Err(err) => Err(err),
    }
}
