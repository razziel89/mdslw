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

fn process(text: &String, max_width: &Option<usize>, end_markers: &String) -> String {
    let parsed = parse(&text);
    let filled = fill_ranges(parsed, &text);
    format(filled, max_width, &end_markers, &text)
}

fn main() {
    // Configure from env vars.
    // Max line length.
    let max_width_num = std::env::var("MDSLW_MAX_WIDTH")
        .unwrap_or("80".to_string())
        .parse::<usize>()
        .expect("max width is a non-negative integer");
    let max_width = if max_width_num == 0 {
        None
    } else {
        Some(max_width_num)
    };
    // Characters that may end sentences.
    let end_markers = std::env::var("MDSLW_END_MARKERS").unwrap_or(".?!:".to_string());

    let text = read_stdin();

    let processed = process(&text, &max_width, &end_markers);

    println!("{}", processed);
}
