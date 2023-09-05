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

use crate::indent::{fill_ranges, spaces, TextRange};
use crate::linebreak::insert_linebreaks_between_sentences;
use crate::parse::parse;

fn read_stdin() -> String {
    std::io::stdin()
        .lines()
        // Ignore lines that cannot be read.
        .filter_map(|el| el.ok())
        .collect::<Vec<_>>()
        .join("\n")
}

fn format(text: &String, ranges: Vec<TextRange>) -> String {
    let mut result = String::new();

    for range in ranges {
        if range.verbatim {
            result.push_str(&text[range.range]);
        } else {
            let indent = spaces(range.indent_spaces);
            result.push_str(&insert_linebreaks_between_sentences(
                &text[range.range],
                &indent,
            ));
        }
    }

    result
}

fn main() {
    let markdown = read_stdin();
    let parsed = parse(&markdown);
    let filled = fill_ranges(parsed, &markdown);

    println!("{}", format(&markdown, filled));
}
