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
mod fs;
mod indent;
mod linebreak;
mod parse;
mod ranges;
mod wrap;

use std::path::PathBuf;

use anyhow::{Context, Error, Result};
use clap::{Parser, ValueEnum};

use crate::call::upstream_formatter;
use crate::fs::find_files_with_extension;
use crate::parse::parse;
use crate::ranges::fill_ranges;
use crate::wrap::format;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OpMode {
    Both,
    Check,
    Format,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Paths to files or directories that shall be processed.
    paths: Vec<PathBuf>,
    /// The maximum line width that is acceptable. A value of 0 disables wrapping of long lines.
    #[arg(short = 'w', long, env = "MDSLW_MAX_WIDTH", default_value_t = 80)]
    max_width: usize,
    /// A set of characters that are acceptable end of line markers.
    #[arg(short, long, env = "MDSLW_END_MARKERS", default_value_t = String::from("?!:."))]
    end_markers: String,
    /// Specify an upstream auto-formatter (with args) that reads from stdin and writes to stdout.
    /// It will be called before mdslw will run. Useful if you want to chain multiple tools. For
    /// example, specify "prettier --parser=markdown" to call prettier first. Run in each file's
    /// directory if PATHS are specified.
    #[arg(short, long, env = "MDSLW_UPSTREAM")]
    upstream: Option<String>,
    /// Mode of operation: check = exit with error if format has to be adjusted but do not format,
    /// format = format the file and exit with error in case of problems only, both = do both
    /// (useful as pre-commit hook).
    #[arg(value_enum, short, long, env = "MDSLW_MODE", default_value_t = OpMode::Format)]
    mode: OpMode,
}

fn read_stdin() -> String {
    std::io::stdin()
        .lines()
        // Ignore lines that cannot be read.
        .filter_map(|el| el.ok())
        .collect::<Vec<_>>()
        .join("\n")
}

fn get_cwd() -> Result<PathBuf> {
    std::env::current_dir()
        .context("getting current working directory")
        .map(|el| el.as_path().to_owned())
        .and_then(|el| std::fs::canonicalize(el).context("canonicalising path"))
}

fn process(
    text: String,
    file_dir: PathBuf,
    upstream: &Option<String>,
    max_width: &Option<usize>,
    end_markers: &String,
) -> Result<String> {
    let after_upstream = if let Some(upstream) = upstream {
        upstream_formatter(&upstream, text, file_dir)?
    } else {
        text
    };

    let parsed = parse(&after_upstream);
    let filled = fill_ranges(parsed, &after_upstream);
    let formatted = format(filled, max_width, &end_markers, &after_upstream);

    Ok(formatted)
}

pub fn get_file_content_and_dir(path: &PathBuf) -> Result<(String, PathBuf)> {
    let text = std::fs::read_to_string(&path).context("failed to read file")?;
    let dir = path
        .parent()
        .ok_or(Error::msg("failed to determine parent directory"))?
        .to_path_buf();

    Ok((text, dir))
}

fn main() -> Result<()> {
    let cli = Args::parse();

    let max_width = if cli.max_width == 0 {
        None
    } else {
        Some(cli.max_width)
    };

    let md_files =
        find_files_with_extension(cli.paths, ".md").context("discovering markdown files")?;

    let unchanged = if md_files.len() == 0 {
        // Procss content from stdin and write to stdout.
        let text = read_stdin();
        let cwd = get_cwd()?;

        let processed = process(
            text.clone(),
            cwd,
            &cli.upstream,
            &max_width,
            &cli.end_markers,
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
        // Process all MD files we found and abort on any error. We will update files in-place.
        let mut has_changed = false;

        for file in md_files {
            let context = || format!("processing markdown file: {}", file.to_string_lossy());

            let (text, dir) = get_file_content_and_dir(&file).with_context(&context)?;

            let processed = process(
                text.clone(),
                dir,
                &cli.upstream,
                &max_width,
                &cli.end_markers,
            )
            .with_context(&context)?;

            // Decide whether to overwrite existing files.
            match cli.mode {
                OpMode::Format | OpMode::Both => {
                    std::fs::write(&file, processed.as_bytes()).with_context(&context)?;
                }
                // Do not write anything in check mode.
                OpMode::Check => {}
            }

            has_changed = has_changed || processed != text;
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
