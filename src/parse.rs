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

use core::ops::Range;
use pulldown_cmark::{Event, Options, Parser, Tag};
use std::collections::HashMap;

use crate::detect::WhitespaceDetector;
use crate::ignore::IgnoreByHtmlComment;

/// CharRange describes a range of characters in a document.
pub type CharRange = Range<usize>;

#[derive(Debug, PartialEq)]
pub struct ParseCfg {
    pub keep_inline_html: bool,
    pub keep_footnotes: bool,
    pub keep_tasklists: bool,
    pub keep_tables: bool,
    pub keep_nbsp: bool,
}

/// Determine ranges of characters that shall later be wrapped and have their indents fixed.
pub fn parse_markdown(text: &str, parse_cfg: &ParseCfg) -> Vec<CharRange> {
    // Enable some options by default to support parsing common kinds of documents.
    let mut opts = Options::empty();
    // If we do not want to modify some elements, we detect them with the parser and consider them
    // as verbatim in the function "to_be_wrapped".
    if parse_cfg.keep_tables {
        opts.insert(Options::ENABLE_TABLES);
    }
    if parse_cfg.keep_footnotes {
        opts.insert(Options::ENABLE_FOOTNOTES);
    }
    if parse_cfg.keep_tasklists {
        opts.insert(Options::ENABLE_TASKLISTS);
    }
    // Do not enable other options:
    // opts.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    // opts.insert(Options::ENABLE_SMART_PUNCTUATION);
    // opts.insert(Options::ENABLE_STRIKETHROUGH);
    let events_and_ranges = Parser::new_ext(text, opts)
        .into_offset_iter()
        .collect::<Vec<_>>();
    let whitespaces = whitespace_indices(text, &WhitespaceDetector::new(parse_cfg.keep_nbsp));

    merge_ranges(
        to_be_wrapped(events_and_ranges, &whitespaces, parse_cfg),
        &whitespaces,
    )
}

/// Filter out those ranges of text that shall be wrapped. See comments in the function for
/// what sections are handled in which way.
fn to_be_wrapped(
    events: Vec<(Event, CharRange)>,
    whitespaces: &HashMap<usize, char>,
    feature_cfg: &ParseCfg,
) -> Vec<CharRange> {
    let mut verbatim_level: usize = 0;
    let mut ignore = IgnoreByHtmlComment::new();

    events
        .into_iter()
        // Mark every range that is between two ignore directives as verbatim by filtering it out.
        .filter(|(event, _range)| {
            if let Event::Html(s) = event {
                ignore.process_html(s)
            }
            !ignore.should_be_ignored()
        })
        .filter(|(event, range)| match event {
            Event::Start(tag) => {
                match tag {
                    // Most delimited blocks should stay as they are. Introducing line breaks would
                    // cause problems here.
                    Tag::BlockQuote
                    | Tag::CodeBlock(..)
                    | Tag::FootnoteDefinition(..)
                    | Tag::Heading(..)
                    | Tag::Image(..)
                    | Tag::Table(..)
                    | Tag::TableCell
                    | Tag::TableHead
                    | Tag::TableRow => {
                        verbatim_level += 1;
                        false
                    }
                    // In case of some blocks, we do not want to extract the text contained inside
                    // them but keep everything the block encompasses.
                    Tag::Emphasis | Tag::Link(..) | Tag::Strikethrough | Tag::Strong => {
                        verbatim_level += 1;
                        true
                    }
                    // Other delimited blocks can be both, inside a verbatim block or inside text.
                    // However, the text they embrace is the important bit but we do not want to
                    // extract the entire range.
                    Tag::Item | Tag::List(..) | Tag::Paragraph => false,
                }
            }

            Event::End(tag) => {
                match tag {
                    // Kept as they were.
                    Tag::BlockQuote
                    | Tag::CodeBlock(..)
                    | Tag::FootnoteDefinition(..)
                    | Tag::Heading(..)
                    | Tag::Image(..)
                    | Tag::Table(..)
                    | Tag::TableCell
                    | Tag::TableHead
                    | Tag::TableRow => {
                        verbatim_level = verbatim_level
                            .checked_sub(1)
                            .expect("tags should be balanced");
                        false
                    }
                    // Should be wrapped but text not extracted.
                    Tag::Emphasis | Tag::Link(..) | Tag::Strikethrough | Tag::Strong => {
                        verbatim_level = verbatim_level
                            .checked_sub(1)
                            .expect("tags should be balanced");
                        false
                    }

                    // Can be anything.
                    Tag::Item | Tag::List(..) | Tag::Paragraph => false,
                }
            }

            // More elements that are not blocks and that should be taken verbatim.
            Event::TaskListMarker(..) | Event::FootnoteReference(..) | Event::Rule => false,

            // Allow editing HTML only if it is inline, i.e. if the range containing the HTML
            // contains no whitespace. Treat it like text in that case.
            Event::Html(..) => {
                !feature_cfg.keep_inline_html
                    && !range
                        .clone()
                        .filter_map(|el| whitespaces.get(&el))
                        .any(|el| el == &'\n')
            }

            // The following should be wrapped if they are not inside a verbatim block. Note that
            // that also includes blocks that are extracted in their enirey (e.g. links). In the
            // context of text contained within, they cound as verbatim blocks, too.
            Event::SoftBreak | Event::HardBreak | Event::Text(..) | Event::Code(..) => {
                verbatim_level == 0
            }
        })
        .map(|(_event, range)| range)
        .collect::<Vec<_>>()
}

/// Check whether there is nothing but whitespace between the end of the previous range and the
/// start of the next one, if the ranges do not connect directly anyway. Note that we still keep
/// paragraphs separated by keeping ranges separate that are separated by more linebreaks than one.
fn merge_ranges(ranges: Vec<CharRange>, whitespaces: &HashMap<usize, char>) -> Vec<CharRange> {
    let mut next_range: Option<CharRange> = None;
    let mut merged = vec![];

    for range in ranges {
        if let Some(next) = next_range {
            let contains_just_whitespace =
                (next.end..range.start).all(|el| whitespaces.contains_key(&el));
            let at_most_one_linebreak = (next.end..range.start)
                .filter(|el| Some(&'\n') == whitespaces.get(el))
                .count()
                <= 1;
            let is_contained = range.start >= next.start && range.end <= next.end;

            if is_contained {
                // Skip the range if it is already included.
                next_range = Some(next);
            } else if contains_just_whitespace && at_most_one_linebreak {
                // Extend the range.
                next_range = Some(CharRange {
                    start: next.start,
                    end: range.end,
                });
            } else {
                // Remember the range and continue extending.
                merged.push(next);
                next_range = Some(range);
            }
        } else {
            next_range = Some(range);
        }
    }

    // Treat the last range that may be left.
    if let Some(next) = next_range {
        merged.push(next)
    }

    // Remove ranges that contain at most 1 character. They never have to be wrapped.
    merged
        .into_iter()
        .filter(|el| el.len() > 1)
        .collect::<Vec<_>>()
}

/// Get all indices that point to whitespace as well as the characters they point to.
fn whitespace_indices(text: &str, detector: &WhitespaceDetector) -> HashMap<usize, char> {
    text.char_indices()
        .filter_map(|(pos, ch)| {
            if detector.is_whitespace(&ch) {
                Some((pos, ch))
            } else {
                None
            }
        })
        .collect::<HashMap<_, _>>()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn detect_whitespace() {
        let text = "some test with witespace at 	some\nlocations";
        let detected = whitespace_indices(text, &WhitespaceDetector::default());
        let expected = vec![
            (4, ' '),
            (9, ' '),
            (14, ' '),
            (24, ' '),
            (27, ' '),
            (28, '\t'),
            (33, '\n'),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();

        assert_eq!(expected, detected);
    }

    #[test]
    fn merging_ranges() {
        let ranges = vec![
            CharRange { start: 0, end: 4 },
            CharRange { start: 5, end: 9 },
            CharRange { start: 11, end: 15 },
            CharRange { start: 11, end: 14 },
            CharRange { start: 16, end: 19 },
            CharRange { start: 23, end: 36 },
        ];
        let whitespace = whitespace_indices(
            "some text\n\nmore text | even more text",
            &WhitespaceDetector::default(),
        );

        let merged = merge_ranges(ranges, &whitespace);

        let expected = vec![
            CharRange { start: 0, end: 9 },
            CharRange { start: 11, end: 19 },
            CharRange { start: 23, end: 36 },
        ];

        assert_eq!(expected, merged);
    }

    #[test]
    fn parsing_markdown() {
        let text = r#"
## Some Heading

Some text.

<!-- some html -->

- More text.
- More text.
  - Even more text.
  - Some text with a [link].

```code
some code
```

[link]: https://something.com "some link"
"#;
        let cfg = ParseCfg {
            keep_inline_html: false,
            keep_footnotes: false,
            keep_tasklists: false,
            keep_tables: false,
            keep_nbsp: false,
        };
        let parsed = parse_markdown(text, &cfg);

        // [18..28, 52..62, 65..75, 80..95, 100..124]
        let expected = vec![
            CharRange { start: 18, end: 28 },
            CharRange { start: 52, end: 62 },
            CharRange { start: 65, end: 75 },
            CharRange { start: 80, end: 95 },
            CharRange {
                start: 100,
                end: 124,
            },
        ];

        assert_eq!(expected, parsed);
    }
}
