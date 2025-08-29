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

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, Error, Result};
use clap::{CommandFactory, Parser};
use clap_complete::generate;
use rayon::prelude::*;

const CONFIG_FILE: &str = ".mdslw.toml";
const YAML_CONFIG_KEY: &str = "mdslw-toml";

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

struct Processor {
    feature_cfg: features::FeatureCfg,
    detector: detect::BreakDetector,
    max_width: Option<usize>,
}

impl Processor {
    fn process(&self, text: String, width_reduction: usize) -> String {
        // At first, process all block quotes.
        let text = if self.feature_cfg.format_block_quotes {
            log::debug!("formatting text in block quotes");
            parse::BlockQuotes::new(&text)
                .apply_to_matches_and_join(|t, indent| self.process(t, indent + width_reduction))
        } else {
            log::debug!("not formatting text in block quotes");
            text
        };
        // Then process the actual text.
        let ends_on_linebreak = text.ends_with('\n');
        let text = if self.feature_cfg.keep_spaces_in_links {
            log::debug!("not replacing spaces in links by non-breaking spaces");
            text
        } else {
            log::debug!("replacing spaces in links by non-breaking spaces");
            replace::replace_spaces_in_links_by_nbsp(text)
        };
        let text = if self.feature_cfg.outsource_inline_links {
            log::debug!("outsourcing inline links");
            replace::outsource_inline_links(
                text,
                &self.feature_cfg.collate_link_defs,
                &self.detector.whitespace,
            )
        } else {
            log::debug!("not outsourcing inline links");
            text
        };
        let text = if self.feature_cfg.collate_link_defs {
            log::debug!("collating links at the end of the document");
            replace::collate_link_defs_at_end(text, &self.detector.whitespace)
        } else {
            log::debug!("not collating links at the end of the document");
            text
        };
        let parsed = parse::parse_markdown(&text, &self.feature_cfg.parse_cfg);
        let filled = ranges::fill_markdown_ranges(parsed, &text);
        let width = &self
            .max_width
            .map(|el| el.checked_sub(width_reduction).unwrap_or(el));
        let formatted = wrap::add_linebreaks_and_wrap(filled, width, &self.detector, &text);

        // Keep newlines at the end of the file in tact. They disappear sometimes.
        let file_end = if !formatted.ends_with('\n') && ends_on_linebreak {
            log::debug!("adding missing trailing newline character");
            "\n"
        } else {
            ""
        };
        format!("{}{}", formatted, file_end)
    }
}

fn process(document: String, file_dir: &Path, cfg: &cfg::PerFileCfg) -> Result<(String, String)> {
    // Prepare user-configured options. These could be outsourced if we didn't intend to allow
    // per-file configurations.
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
    let processor = Processor {
        feature_cfg,
        detector,
        max_width,
    };

    // Actually process the text.
    let frontmatter = frontmatter::extract_frontmatter(&document);
    let text = document[frontmatter.len()..].to_string();

    let after_upstream = if !cfg.upstream.is_empty() {
        log::debug!("calling upstream formatter: {}", cfg.upstream);
        call::upstream_formatter(&cfg.upstream, text, file_dir)?
    } else {
        log::debug!("not calling any upstream formatter");
        text
    };

    let processed = format!("{}{}", frontmatter, processor.process(after_upstream, 0));
    Ok((processed, document))
}

fn process_stdin<F>(mode: &cfg::OpMode, build_cfg: F, file_path: &PathBuf) -> Result<bool>
where
    F: Fn(&str, &PathBuf) -> Result<cfg::PerFileCfg>,
{
    log::debug!("processing content from stdin and writing to stdout");
    let text = fs::read_stdin();

    let config = build_cfg(&text, file_path).context("failed to build complete config")?;

    let file_dir = file_path
        .parent()
        .map(|el| el.to_path_buf())
        .unwrap_or(PathBuf::from("."));
    let (processed, text) = process(text, file_dir.as_path(), &config)?;

    // Decide what to output.
    match mode {
        cfg::OpMode::Format | cfg::OpMode::Both => {
            log::debug!("writing modified file to stdout");
            print!("{}", processed);
        }
        cfg::OpMode::Check => {
            log::debug!("writing original file to stdout in check mode");
            print!("{}", text);
        }
    }

    Ok(processed == text)
}

fn process_file<F>(mode: &cfg::OpMode, path: &PathBuf, build_cfg: F) -> Result<(String, String)>
where
    F: Fn(&str, &PathBuf) -> Result<cfg::PerFileCfg>,
{
    let report_path = path.to_string_lossy();
    log::debug!("processing {}", report_path);

    let (text, file_dir) = fs::get_file_content_and_dir(path)?;
    let config = build_cfg(&text, path).context("failed to build complete config")?;
    let (processed, text) = process(text, &file_dir, &config)?;

    // Decide whether to overwrite existing files.
    match mode {
        cfg::OpMode::Format | cfg::OpMode::Both => {
            if processed == text {
                log::debug!("keeping OK file {}", report_path);
            } else {
                log::debug!("modifying NOK file {} in place", report_path);
                std::fs::write(path, processed.as_bytes()).context("failed to write file")?;
            }
        }
        // Do not write anything in check mode.
        cfg::OpMode::Check => {
            log::debug!("not modifying file {} in check mode", report_path);
        }
    }

    Ok((processed, text))
}

fn read_config_file(path: &Path) -> Option<(PathBuf, cfg::CfgFile)> {
    let result = std::fs::read_to_string(path)
        .context("failed to read file")
        .and_then(|el| {
            toml::from_str::<cfg::CfgFile>(&el).context("that failed to parse due to error:")
        });

    match result {
        Ok(cfg) => {
            log::debug!("parsed config file {}", path.to_string_lossy());
            Some((path.to_path_buf(), cfg))
        }
        Err(err) => {
            log::error!("ignoring config file {} {:?}", path.to_string_lossy(), err);
            None
        }
    }
}

fn build_document_specific_config(
    document: &str,
    document_path: &Path,
    cli: &cfg::CliArgs,
    configs: &Vec<(PathBuf, cfg::CfgFile)>,
) -> Result<cfg::PerFileCfg> {
    let config_from_frontmatter =
        toml::from_str::<cfg::CfgFile>(&parse::simply_get_value_for_yaml_key(
            &frontmatter::extract_frontmatter(document),
            YAML_CONFIG_KEY,
        ))
        .with_context(|| {
            format!(
                "failed to parse frontmatter entry as toml config:\n{}",
                document
            )
        })?;
    let config_tuple = [(document_path.to_path_buf(), config_from_frontmatter)];
    Ok(cfg::merge_configs(cli, config_tuple.iter().chain(configs)))
}

fn print_config_file() -> Result<()> {
    toml::to_string(&cfg::CfgFile::default())
        .context("converting to toml format")
        .map(|cfg| println!("{}", cfg))
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
    // Generation of default config file.
    if cli.default_config {
        log::info!("writing default config file to stdout");
        return print_config_file();
    }

    // All other actions could technically be specified on a per-file level.
    let cwd = PathBuf::from(".");
    let unchanged = if cli.paths.is_empty() {
        let file_path = cli.stdin_filepath.clone().unwrap_or(PathBuf::from("STDIN"));
        let file_dir = file_path.parent().unwrap_or(cwd.as_path());
        let configs = fs::find_files_upwards(file_dir, CONFIG_FILE, &mut None)
            .into_iter()
            .filter_map(|el| read_config_file(&el))
            .collect::<Vec<_>>();
        let build_document_config = |document: &str, file_path: &PathBuf| {
            build_document_specific_config(document, file_path, &cli, &configs)
        };
        process_stdin(&cli.mode, build_document_config, &file_path)
    } else {
        let md_files = fs::find_files_with_extension(&cli.paths, &cli.extension)
            .context("failed to discover markdown files")?;
        log::debug!("will process {} markdown file(s) from disk", md_files.len());
        let config_files = {
            // Define a temporary cache to avoid scanning the same directories again and again.
            let mut cache = Some(HashSet::new());
            md_files
                .iter()
                .flat_map(|el| fs::find_files_upwards(el, CONFIG_FILE, &mut cache))
                .filter_map(|el| read_config_file(&el))
                .collect::<HashMap<_, _>>()
        };
        log::debug!("loaded {} configs from disk", config_files.len());

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
            log::debug!("disabling possibly set diff pager for non-diff report");
            &None
        };
        let par_printer = call::ParallelPrinter::new(diff_pager)?;

        // Process all MD files we found.
        md_files
            .par_iter()
            .map(|path| {
                log::info!("processing markdown file {}", path.to_string_lossy());
                let configs = fs::UpwardsDirsIterator::new(path)
                    .filter_map(|el| {
                        config_files
                            .get(&el.join(CONFIG_FILE))
                            .map(|cfg| (el, cfg.clone()))
                    })
                    .collect::<Vec<_>>();
                let build_document_config = |document: &str, file_path: &PathBuf| {
                    build_document_specific_config(document, file_path, &cli, &configs)
                };
                match process_file(&cli.mode, path, build_document_config) {
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
                }
            })
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
