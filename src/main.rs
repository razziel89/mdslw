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
mod indent;
mod linebreak;
mod parse;
mod ranges;
mod wrap;

use anyhow::Result;
use clap::Parser;

use crate::parse::parse;
use crate::ranges::fill_ranges;
use crate::wrap::format;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Paths to files or directories that shall be processed.
    paths: Vec<String>,
    /// The maximum line width that is acceptable. A value of 0 disables line wrapping.
    #[arg(short, long, env, default_value_t = 80)]
    max_width: usize,
    /// A set of characters that are acceptable end of line markers.
    #[arg(short, long, env, default_value_t = String::from("?!:."))]
    end_markers: String,
}

fn read_stdin() -> String {
    std::io::stdin()
        .lines()
        // Ignore lines that cannot be read.
        .filter_map(|el| el.ok())
        .collect::<Vec<_>>()
        .join("\n")
}

fn process(text: &String, max_width: &Option<usize>, end_markers: &String) -> String {
    let parsed = parse(&text);
    let filled = fill_ranges(parsed, &text);
    format(filled, max_width, &end_markers, &text)
}

fn main() -> Result<()> {
    let cli = Args::parse();

    let max_width = if cli.max_width == 0 {
        None
    } else {
        Some(cli.max_width)
    };

    let text = read_stdin();

    let processed = process(&text, &max_width, &cli.end_markers);

    println!("{}", processed);

    Ok(())
}
