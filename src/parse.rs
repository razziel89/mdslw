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
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::collections::HashMap;
use std::fmt::Write;

use crate::detect::WhitespaceDetector;
use crate::ignore::IgnoreByHtmlComment;
use crate::trace_log;

/// CharRange describes a range of characters in a document.
pub type CharRange = Range<usize>;

#[derive(Debug, PartialEq)]
pub struct ParseCfg {
    pub keep_linebreaks: bool,
}

/// Determine ranges of characters that shall later be wrapped and have their indents fixed.
pub fn parse_markdown(text: &str, parse_cfg: &ParseCfg) -> Vec<CharRange> {
    // Enable some options by default to support parsing common kinds of documents.
    let mut opts = Options::empty();
    // If we do not want to modify some elements, we detect them with the parser and consider them
    // as verbatim in the function "to_be_wrapped".
    log::debug!("detecting tables");
    opts.insert(Options::ENABLE_TABLES);
    // Do not enable other options:
    // opts.insert(Options::ENABLE_FOOTNOTES);
    // opts.insert(Options::ENABLE_TASKLISTS);
    // opts.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    // opts.insert(Options::ENABLE_SMART_PUNCTUATION);
    // opts.insert(Options::ENABLE_STRIKETHROUGH);
    let events_and_ranges = Parser::new_ext(text, opts)
        .into_offset_iter()
        .collect::<Vec<_>>();
    let whitespaces = whitespace_indices(text, &WhitespaceDetector::new(parse_cfg.keep_linebreaks));

    merge_ranges(to_be_wrapped(events_and_ranges, &whitespaces), &whitespaces)
}

/// Filter out those ranges of text that shall be wrapped. See comments in the function for
/// what sections are handled in which way.
fn to_be_wrapped(
    events: Vec<(Event, CharRange)>,
    whitespaces: &HashMap<usize, char>,
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
                    | Tag::Heading { .. }
                    | Tag::Image { .. }
                    | Tag::Table(..)
                    | Tag::TableCell
                    | Tag::TableHead
                    | Tag::TableRow => {
                        verbatim_level += 1;
                        false
                    }
                    // In case of some blocks, we do not want to extract the text contained inside
                    // them but keep everything the block encompasses.
                    Tag::Emphasis | Tag::Link { .. } | Tag::Strikethrough | Tag::Strong => {
                        verbatim_level += 1;
                        true
                    }
                    // Other delimited blocks can be both, inside a verbatim block or inside text.
                    // However, the text they embrace is the important bit but we do not want to
                    // extract the entire range.
                    Tag::Item | Tag::List(..) | Tag::Paragraph | Tag::MetadataBlock(..) => false,

                    // See below for why HTML blocks are treated like this.
                    Tag::HtmlBlock => !range
                        .clone()
                        .filter_map(|el| whitespaces.get(&el))
                        .any(|el| el == &'\n'),
                }
            }

            Event::End(tag) => {
                match tag {
                    // Kept as they were.
                    TagEnd::BlockQuote
                    | TagEnd::CodeBlock
                    | TagEnd::FootnoteDefinition
                    | TagEnd::Heading(..)
                    | TagEnd::Image
                    | TagEnd::Table
                    | TagEnd::TableCell
                    | TagEnd::TableHead
                    | TagEnd::TableRow => {
                        verbatim_level = verbatim_level
                            .checked_sub(1)
                            .expect("tags should be balanced");
                        false
                    }
                    // Should be wrapped but text not extracted.
                    TagEnd::Emphasis | TagEnd::Link | TagEnd::Strikethrough | TagEnd::Strong => {
                        verbatim_level = verbatim_level
                            .checked_sub(1)
                            .expect("tags should be balanced");
                        false
                    }

                    // Can be anything.
                    TagEnd::Item
                    | TagEnd::List(..)
                    | TagEnd::Paragraph
                    | TagEnd::HtmlBlock
                    | TagEnd::MetadataBlock(..) => false,
                }
            }

            // More elements that are not blocks and that should be taken verbatim.
            Event::TaskListMarker(..) | Event::FootnoteReference(..) | Event::Rule => false,

            // Allow editing HTML only if it is inline, i.e. if the range containing the HTML
            // contains no whitespace. Treat it like text in that case.
            Event::Html(..) | Event::InlineHtml(..) => !range
                .clone()
                .filter_map(|el| whitespaces.get(&el))
                .any(|el| el == &'\n'),

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

#[derive(Debug)]
enum RangeMatch<'a> {
    Matches(&'a str),
    NoMatch(&'a str),
}

pub struct BlockQuotes<'a>(Vec<RangeMatch<'a>>);

impl<'a> BlockQuotes<'a> {
    pub const FULL_PREFIX: &'static str = "> ";
    pub const FULL_PREFIX_LEN: usize = Self::FULL_PREFIX.len();
    pub const SHORT_PREFIX: &'static str = ">";

    fn strip_prefix(text: &str) -> String {
        text.split_inclusive('\n')
            .map(|t| {
                t.strip_prefix(Self::SHORT_PREFIX)
                    .map(|el| el.strip_prefix(' ').unwrap_or(el))
                    .unwrap_or(t)
            })
            .collect::<String>()
    }

    fn add_prefix(text: String) -> String {
        // The "write!" calls should never fail since we write to a String that we create here.
        let mut result = String::from("\n");
        text.split_inclusive('\n').for_each(|line| {
            let prefix = if line.len() == 1 {
                Self::SHORT_PREFIX
            } else {
                Self::FULL_PREFIX
            };
            write!(result, "{}{}", prefix, line).expect("building block-quote formated result");
        });
        writeln!(result).expect("building block-quote formated result");
        result
    }

    pub fn new(text: &'a str) -> Self {
        let mut level: usize = 0;
        // In case we ever need to iterate over other kinds of syntax, the tag as well as the
        // function stripping prefixes will have to be adjusted.
        let tag = Tag::BlockQuote;

        let mut opts = Options::empty();
        opts.insert(Options::ENABLE_TABLES);
        opts.insert(Options::ENABLE_FOOTNOTES);
        opts.insert(Options::ENABLE_TASKLISTS);
        opts.insert(Options::ENABLE_HEADING_ATTRIBUTES);
        opts.insert(Options::ENABLE_SMART_PUNCTUATION);
        opts.insert(Options::ENABLE_STRIKETHROUGH);

        let ranges = Parser::new_ext(text, opts)
            .into_offset_iter()
            .filter_map(|(event, range)| match event {
                Event::Start(start) => {
                    level += 1;
                    if level == 1 {
                        Some((start == tag, range))
                    } else {
                        None
                    }
                }
                Event::End(_) => {
                    level -= 1;
                    None
                }
                _ => {
                    if level == 0 {
                        Some((false, range))
                    } else {
                        None
                    }
                }
            })
            .map(|(matches, range)| {
                if matches {
                    RangeMatch::Matches(&text[range])
                } else {
                    RangeMatch::NoMatch(&text[range])
                }
            })
            .collect::<Vec<_>>();

        Self(ranges)
    }

    pub fn apply_to_matches_and_join<MapFn>(self, func: MapFn) -> String
    where
        MapFn: Fn(String) -> String,
    {
        self.0
            .into_iter()
            .map(|el| match el {
                RangeMatch::NoMatch(s) => s.to_string(),
                RangeMatch::Matches(s) => Self::add_prefix(func(Self::strip_prefix(s))),
            })
            .collect::<String>()
    }
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
    let removed = merged
        .into_iter()
        .filter(|el| el.len() > 1)
        .collect::<Vec<_>>();

    trace_log!(
        "formattable byte ranges: {}",
        removed
            .iter()
            .map(|range| format!("[{},{})", range.start, range.end))
            .collect::<Vec<_>>()
            .join(" ")
    );

    removed
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
            keep_linebreaks: false,
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
