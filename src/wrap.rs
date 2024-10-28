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

use crate::detect::{BreakDetector, WhitespaceDetector};
use crate::indent::build_indent;
use crate::linebreak::insert_linebreaks_after_sentence_ends;
use crate::ranges::{TextRange, WrapType};
use crate::trace_log;

pub fn add_linebreaks_and_wrap(
    ranges: Vec<TextRange>,
    max_width: &Option<usize>,
    detector: &BreakDetector,
    text: &str,
) -> String {
    let mut result = String::new();

    for range in ranges {
        if let WrapType::Indent(indent_spaces) = range.wrap {
            trace_log!(
                "wrapping text: {}",
                text[range.range.clone()].replace('\n', "\\n")
            );
            let indent = build_indent(indent_spaces);
            trace_log!("keeping indent in mind: '{}'", indent);
            let broken = insert_linebreaks_after_sentence_ends(&text[range.range], detector);
            trace_log!(
                "with linebreaks after sentences: {}",
                broken.replace('\n', "\\n")
            );
            let wrapped = broken
                .split('\n')
                .enumerate()
                .flat_map(|(idx, el)| {
                    wrap_long_line_and_collapse_inline_whitespace(
                        el,
                        idx,
                        max_width,
                        &indent,
                        &detector.whitespace,
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            trace_log!(
                "after wrapping long sentences: {}",
                wrapped.replace('\n', "\\n")
            );
            result.push_str(&wrapped);
        } else {
            trace_log!(
                "keeping text: {}",
                text[range.range.clone()].to_string().replace('\n', "\\n")
            );
            result.push_str(&text[range.range]);
        }
    }

    result.trim_end().to_string()
}

/// The main purpose of this function is to wrap a long line, making sure to add the linebreak
/// between words. It does so by splitting by whitespace and then joining again by spaces. One side
/// effect that we accept here is that all consecutive inline whitespace will be replaced by a
/// single space due to the splitting-and-joining process.
fn wrap_long_line_and_collapse_inline_whitespace(
    sentence: &str,
    sentence_idx: usize,
    max_width: &Option<usize>,
    indent: &str,
    detector: &WhitespaceDetector,
) -> Vec<String> {
    let mut lines = vec![];
    let mut words = detector
        .split_whitespace(sentence)
        .filter(|el| !el.is_empty());
    let (mut line, first_indent_len) = if let Some(first_word) = words.next() {
        // The first sentence is already properly indented. Every other sentence has to be
        // indented manually.
        if sentence_idx == 0 {
            (String::from(first_word), indent.chars().count())
        } else {
            (format!("{}{}", indent, first_word), 0)
        }
    } else {
        (String::new(), 0)
    };
    let mut line_len = line.chars().count() + first_indent_len;
    let width = max_width.unwrap_or(0);
    for word in words {
        let chars = word.chars().count();
        if width == 0 || line_len + 1 + chars <= width {
            line.push(' ');
            line.push_str(word);
            line_len += chars + 1;
        } else {
            lines.push(line);
            line = String::from(indent);
            line.push_str(word);
            line_len = line.chars().count();
        }
    }
    lines.push(line);
    lines
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::detect::BreakCfg;
    use crate::parse::CharRange;

    const CFG_FOR_TESTS: &BreakCfg = &BreakCfg {
        keep_linebreaks: false,
    };

    #[test]
    fn wrapping_long_sentence() {
        let sentence = "this sentence is not that long but will be wrapped";
        let sentence_idx = 0;
        let max_width = 11;
        let indent = "  ";
        let wrapped = wrap_long_line_and_collapse_inline_whitespace(
            sentence,
            sentence_idx,
            &Some(max_width),
            indent,
            &WhitespaceDetector::default(),
        );

        // No indent for the start of the sentence due to the sentence_idx.
        let expected = vec![
            "this",
            "  sentence",
            "  is not",
            "  that long",
            "  but will",
            "  be",
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
        let wrapped = wrap_long_line_and_collapse_inline_whitespace(
            sentence,
            sentence_idx,
            &Some(max_width),
            indent,
            &WhitespaceDetector::default(),
        );

        // Note the indent for the start of the sentence due to the sentence_idx.
        let expected = vec!["|some", "|sentence", "|with", "|words"];

        assert_eq!(expected, wrapped);
    }

    #[test]
    fn not_wrapping_long_sentence_unless_requested() {
        let sentence = "this sentence is somewhat long but will not be wrapped";
        let sentence_idx = 0;
        let indent = "  ";
        let wrapped = wrap_long_line_and_collapse_inline_whitespace(
            sentence,
            sentence_idx,
            &None,
            indent,
            &WhitespaceDetector::default(),
        );

        let expected = vec![sentence];

        assert_eq!(expected, wrapped);
    }

    #[test]
    fn adding_linebreaks_after_sentences() {
        let ranges = vec![
            TextRange {
                wrap: WrapType::Indent(0),
                range: CharRange { start: 0, end: 33 },
            },
            // The pipe should remain verbatim.
            TextRange {
                wrap: WrapType::Verbatim,
                range: CharRange { start: 33, end: 36 },
            },
            TextRange {
                wrap: WrapType::Indent(3),
                range: CharRange { start: 36, end: 74 },
            },
        ];
        let text = String::from(
            "Some text. It contains sentences. | It's separated in two. Parts, that is.",
        );
        let detector = BreakDetector::new("", "", false, ".", CFG_FOR_TESTS);

        let wrapped = add_linebreaks_and_wrap(ranges, &None, &detector, &text);

        // Whitespace at the start of a range is also merged into one space. Not sure if that makes
        // sense but it does not appear to be relevant in practice, probably due to the way we
        // parse the markdown files. That is, none of the ranges we get appear to start with
        // whitespace at all.
        let expected = String::from(
            "Some text.\nIt contains sentences. | It's separated in two.\n   Parts, that is.",
        );
        assert_eq!(expected, wrapped);
    }

    #[test]
    fn adding_linebreaks_after_sentences_with_keep_words() {
        let ranges = vec![TextRange {
            wrap: WrapType::Indent(0),
            range: CharRange { start: 0, end: 33 },
        }];
        let text = String::from("Some text. It contains sentences.");
        let detector = BreakDetector::new("TEXT.", "", false, ".", CFG_FOR_TESTS);

        let wrapped = add_linebreaks_and_wrap(ranges, &None, &detector, &text);

        let expected = String::from("Some text. It contains sentences.");
        assert_eq!(expected, wrapped);
    }
}
