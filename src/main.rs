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

mod indent;
mod linebreak;
mod parse;
mod ranges;
mod wrap;

use crate::parse::parse;
use crate::ranges::fill_ranges;
use crate::wrap::format;

fn read_stdin() -> String {
    std::io::stdin()
        .lines()
        // Ignore lines that cannot be read.
        .filter_map(|el| el.ok())
        .collect::<Vec<_>>()
        .join("\n")
}

fn main() {
    // Configure from env vars.
    let max_width = std::env::var("MDSLW_MAX_WIDTH")
        .unwrap_or("80".to_string())
        .parse::<usize>()
        .expect("max width is a non-negative integer");
    let end_markers = std::env::var("MDSLW_END_MARKERS").unwrap_or(".?!:".to_string());

    let markdown = read_stdin();

    let parsed = parse(&markdown);
    let filled = fill_ranges(parsed, &markdown);
    let formatted = format(filled, Some(max_width), &end_markers, &markdown);

    println!("{}", formatted);
}
