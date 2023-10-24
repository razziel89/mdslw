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
use crate::keep::KeepWords;
use crate::linebreak::insert_linebreaks_between_sentences;
use crate::ranges::TextRange;

pub fn add_linebreaks_and_wrap(
    ranges: Vec<TextRange>,
    max_width: &Option<usize>,
    end_markers: &str,
    keep_words: &KeepWords,
    text: &String,
) -> String {
    let mut result = String::new();

    for range in ranges {
        if range.verbatim {
            result.push_str(&text[range.range]);
        } else {
            let indent = build_indent(range.indent_spaces);
            let broken = insert_linebreaks_between_sentences(
                &text[range.range],
                &indent,
                end_markers,
                keep_words,
            );
            let wrapped = broken
                .split("\n")
                .enumerate()
                .flat_map(|(idx, el)| wrap_long_sentence(el, idx, max_width, &indent))
                .collect::<Vec<_>>()
                .join("\n");
            result.push_str(&wrapped);
        }
    }

    result.trim_end().to_string()
}

fn wrap_long_sentence(
    sentence: &str,
    sentence_idx: usize,
    max_width: &Option<usize>,
    indent: &str,
) -> Vec<String> {
    if let Some(width) = *max_width {
        let mut lines = vec![];
        let mut words = sentence.split_whitespace();
        let (mut line, first_indent_len) = if let Some(first_word) = words.next() {
            // The first sentence is already properly indented. Every other sentence has to be
            // indented manually.
            if sentence_idx == 0 {
                (String::from(first_word), indent.len())
            } else {
                (format!("{}{}", indent, first_word), 0)
            }
        } else {
            (String::new(), 0)
        };
        for word in words {
            if first_indent_len + line.len() + 1 + word.len() <= width {
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::parse::CharRange;

    #[test]
    fn wrapping_long_sentence() {
        let sentence = "this sentence is not that long but will be wrapped";
        let sentence_idx = 0;
        let max_width = 11;
        let indent = "  ";
        let wrapped = wrap_long_sentence(sentence, sentence_idx, &Some(max_width), indent);

        // No indent for the start of the sentence due to the sentence_idx.
        let expected = vec![
            "this",
            "  sentence",
            "  is not",
            "  that",
            "  long",
            "  but",
            "  will be",
            "  wrapped",
        ];

        assert_eq!(expected, wrapped);
    }

    #[test]
    fn wrapping_long_sentence_that_is_not_the_first() {
        let sentence = "some sentence with words";
        let sentence_idx = 1;
        let max_width = 5;
        // Indent will be copied, does not have to be whitespace.
        let indent = "|";
        let wrapped = wrap_long_sentence(sentence, sentence_idx, &Some(max_width), indent);

        // Note the indent for the start of the sentence due to the sentence_idx.
        let expected = vec!["|some", "|sentence", "|with", "|words"];

        assert_eq!(expected, wrapped);
    }

    #[test]
    fn not_wrapping_long_sentence_unless_requested() {
        let sentence = "this sentence is somewhat long but will not be wrapped";
        let sentence_idx = 2;
        let indent = "  ";
        let wrapped = wrap_long_sentence(sentence, sentence_idx, &None, indent);

        let expected = vec![sentence];

        assert_eq!(expected, wrapped);
    }

    #[test]
    fn adding_linebreaks_after_sentences() {
        let ranges = vec![
            TextRange {
                verbatim: false,
                indent_spaces: 0,
                range: CharRange { start: 0, end: 34 },
            },
            // The pipe should remain verbatim.
            TextRange {
                verbatim: true,
                indent_spaces: 0,
                range: CharRange { start: 33, end: 35 },
            },
            TextRange {
                verbatim: false,
                indent_spaces: 2,
                range: CharRange { start: 35, end: 75 },
            },
        ];
        let text = String::from(
            "Some text. It contains sentences. |  It's separated in two. Parts, that is.",
        );
        let keep = KeepWords::new("", "", false);

        let wrapped = add_linebreaks_and_wrap(ranges, &None, ".", &keep, &text);

        // Whitespace at the start of a range is also merged into one space. Not sure if that makes
        // sense but it does not appear to be relevant in practice, probably due to the way we
        // parse the markdown files. That is, none of the ranges we get appear to start with
        // whitespace at all.
        let expected = String::from(
            "Some text.\nIt contains sentences. | It's separated in two.\n  Parts, that is.",
        );
        assert_eq!(expected, wrapped);
    }

    #[test]
    fn adding_linebreaks_after_sentences_with_keep_words() {
        let ranges = vec![TextRange {
            verbatim: false,
            indent_spaces: 0,
            range: CharRange { start: 0, end: 33 },
        }];
        let text = String::from("Some text. It contains sentences.");
        let keep = KeepWords::new("TEXT.", "", false);

        let wrapped = add_linebreaks_and_wrap(ranges, &None, ".", &keep, &text);

        let expected = String::from("Some text. It contains sentences.");
        assert_eq!(expected, wrapped);
    }
}
