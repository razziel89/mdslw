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

use crate::indent::build_indent;
use crate::linebreak::insert_linebreaks_between_sentences;
use crate::ranges::TextRange;

pub fn format(ranges: Vec<TextRange>, max_width: Option<usize>, text: &String) -> String {
    let mut result = String::new();

    for range in ranges {
        if range.verbatim {
            result.push_str(&text[range.range]);
        } else {
            let indent = build_indent(range.indent_spaces);
            let wrapped = insert_linebreaks_between_sentences(&text[range.range], &indent)
                .split("\n")
                .enumerate()
                .flat_map(|(idx, el)| wrap_sentence(el, idx, max_width, &indent))
                .collect::<Vec<_>>()
                .join("\n");
            result.push_str(&wrapped);
        }
    }

    result.trim_end().to_string()
}

fn wrap_sentence(
    sentence: &str,
    sentence_idx: usize,
    max_width: Option<usize>,
    indent: &str,
) -> Vec<String> {
    if let Some(width) = max_width {
        let mut lines = vec![];
        let mut words = sentence.split_whitespace();
        let mut line = if let Some(first_word) = words.next() {
            // The first sentence is already properly indented. Every other sentence has to be
            // indented manually.
            if sentence_idx == 0 {
                String::from(first_word)
            } else {
                format!("{}{}", indent, first_word)
            }
        } else {
            String::new()
        };
        for word in words {
            if line.len() + 1 + word.len() <= width {
                line.push_str(" ");
                line.push_str(word);
            } else {
                lines.push(line);
                line = String::from(indent);
                line.push_str(word);
            }
        }
        lines.push(line);
        lines
    } else {
        vec![String::from(sentence)]
    }
}
