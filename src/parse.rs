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
use crate::indent::build_indent;
use crate::trace_log;

const YAML_CONFIG_KEY: &str = "mdslw-toml";

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
    log::debug!("detecting definition lists");
    opts.insert(Options::ENABLE_DEFINITION_LIST);
    // Do not enable other options:
    // opts.insert(Options::ENABLE_FOOTNOTES);
    // opts.insert(Options::ENABLE_TASKLISTS);
    // opts.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    // opts.insert(Options::ENABLE_SMART_PUNCTUATION);
    // opts.insert(Options::ENABLE_STRIKETHROUGH);
    let events_and_ranges = Parser::new_ext(text, opts)
        .into_offset_iter()
        .inspect(|(event, range)| {
            trace_log!("parsed [{}, {}): {:?}", range.start, range.end, event)
        })
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
                    Tag::BlockQuote(..)
                    | Tag::CodeBlock(..)
                    | Tag::FootnoteDefinition(..)
                    | Tag::Heading { .. }
                    | Tag::Image { .. }
                    | Tag::Superscript
                    | Tag::Subscript
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
                    Tag::Item
                    | Tag::List(..)
                    | Tag::Paragraph
                    | Tag::MetadataBlock(..)
                    | Tag::DefinitionList
                    | Tag::DefinitionListTitle
                    | Tag::DefinitionListDefinition => false,

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
                    TagEnd::BlockQuote(..)
                    | TagEnd::CodeBlock
                    | TagEnd::FootnoteDefinition
                    | TagEnd::Heading(..)
                    | TagEnd::Superscript
                    | TagEnd::Subscript
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
                    | TagEnd::DefinitionList
                    | TagEnd::DefinitionListTitle
                    | TagEnd::DefinitionListDefinition
                    | TagEnd::Paragraph
                    | TagEnd::HtmlBlock
                    | TagEnd::MetadataBlock(..) => false,
                }
            }

            // More elements that are not blocks and that should be taken verbatim.
            Event::TaskListMarker(..) | Event::FootnoteReference(..) | Event::Rule => false,

            // We do not support detecting math so far as we do not intend to modify match in any
            // way. That is, we treat it as any other text and don't have the parser detect math
            // specifically.
            Event::InlineMath(..) | Event::DisplayMath(..) => false,

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
    Matches((usize, &'a str)),
    NoMatch(&'a str),
}

pub struct BlockQuotes<'a>(Vec<RangeMatch<'a>>);

impl<'a> BlockQuotes<'a> {
    pub const FULL_PREFIX: &'static str = "> ";
    pub const FULL_PREFIX_LEN: usize = Self::FULL_PREFIX.len();
    pub const SHORT_PREFIX: &'static str = ">";

    fn strip_prefix(text: &str, indent: usize) -> String {
        // The first line does start with the actual prefix, while the other lines start with a
        // number of other characters. Thus, we strip the off for all but the first line.
        text.split_inclusive('\n')
            .enumerate()
            .map(|(idx, t)| {
                let t = if idx == 0 { t } else { &t[indent..t.len()] };
                t.strip_prefix(Self::SHORT_PREFIX)
                    .map(|el| el.strip_prefix(' ').unwrap_or(el))
                    .unwrap_or(t)
            })
            .collect::<String>()
    }

    fn add_prefix(text: String, indent: usize) -> String {
        let indent = build_indent(indent);
        // The "write!" calls should never fail since we write to a String that we create here.
        let mut result = String::new();
        text.split_inclusive('\n')
            .enumerate()
            .for_each(|(idx, line)| {
                let prefix = if line.len() == 1 {
                    Self::SHORT_PREFIX
                } else {
                    Self::FULL_PREFIX
                };
                // The first line is already correctly indented. For the other lines, we have to add
                // the indent.
                let ind = if idx == 0 { "" } else { &indent };
                write!(result, "{}{}{}", ind, prefix, line)
                    .expect("building block-quote formated result");
            });
        result
    }

    fn indents(text: &str) -> Vec<usize> {
        text.split_inclusive('\n')
            .flat_map(|line| 0..line.len())
            .collect::<Vec<_>>()
    }

    pub fn new(text: &'a str) -> Self {
        let mut level: usize = 0;
        // In case we ever need to iterate over other kinds of syntax, the tag as well as the
        // function stripping prefixes will have to be adjusted.

        let indents = Self::indents(text);
        let mut start = 0;

        let mut ranges = Parser::new(text)
            .into_offset_iter()
            .filter_map(|(event, range)| match event {
                Event::Start(start) => {
                    if matches!(start, Tag::BlockQuote(..)) {
                        level += 1;
                    }
                    if level == 1 && matches!(start, Tag::BlockQuote(..)) {
                        // Using a CharRange here to prevent the flat_map below from flattening
                        // all the ranges, since Range<usize> supports flattening but our
                        // CharRange does not.
                        Some(CharRange {
                            start: range.start,
                            end: range.end,
                        })
                    } else {
                        None
                    }
                }
                Event::End(end) => {
                    if matches!(end, TagEnd::BlockQuote(..)) {
                        level -= 1;
                    }
                    None
                }
                _ => None,
            })
            .flat_map(|range| {
                let prev_start = start;
                let this_start = range.start;
                start = range.end;

                let this = RangeMatch::Matches((indents[this_start], &text[range]));
                if this_start == prev_start {
                    vec![this]
                } else {
                    let missing = RangeMatch::NoMatch(&text[prev_start..this_start]);
                    vec![missing, this]
                }
            })
            .collect::<Vec<_>>();

        if start != text.len() {
            ranges.push(RangeMatch::NoMatch(&text[start..text.len()]));
        }

        Self(ranges)
    }

    /// The argument `func` should keep a line break at the end if its arguments ends in one. In
    /// most cases, it ends in a line break.
    pub fn apply_to_matches_and_join<MapFn>(self, func: MapFn) -> String
    where
        MapFn: Fn(String, usize) -> String,
    {
        self.0
            .into_iter()
            .map(|el| match el {
                RangeMatch::NoMatch(s) => s.to_string(),
                RangeMatch::Matches(s) => Self::add_prefix(
                    func(Self::strip_prefix(s.1, s.0), s.0 + Self::FULL_PREFIX_LEN),
                    s.0,
                ),
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

enum YAMLBlockStartLineType {
    Pipe,
    Angle,
    None,
}

impl YAMLBlockStartLineType {
    fn is_actual_start_line(&self) -> bool {
        matches!(self, Self::Pipe | Self::Angle)
    }
}

/// Parse a YAML text without an external dependency. We interpret text as being a single YAML
/// document. We search until we find a line starting with the given key. We return everything that
/// is at the same indentation as the line following the key.
pub fn get_value_for_mdslw_toml_yaml_key(text: &str) -> String {
    log::info!(
        "extracting value for key {} from yaml:\n{}",
        YAML_CONFIG_KEY,
        text
    );
    let key = YAML_CONFIG_KEY;
    let key_with_colon = YAML_CONFIG_KEY.to_string() + ":";
    let start_line_type = |line: &str| {
        let split = line.split_whitespace().collect::<Vec<&str>>();
        match split.as_slice() {
            [actual, ":", "|"] | [actual, ":", "|-"] | [actual, ":", "|+"] => {
                if actual == &key {
                    YAMLBlockStartLineType::Pipe
                } else {
                    YAMLBlockStartLineType::None
                }
            }
            [actual, "|"] | [actual, "|-"] | [actual, "|+"] => {
                if actual == &key_with_colon {
                    YAMLBlockStartLineType::Pipe
                } else {
                    YAMLBlockStartLineType::None
                }
            }
            [actual, ":", ">"] | [actual, ":", ">-"] | [actual, ":", ">+"] => {
                if actual == &key {
                    YAMLBlockStartLineType::Angle
                } else {
                    YAMLBlockStartLineType::None
                }
            }
            [actual, ">"] | [actual, ">-"] | [actual, ">+"] => {
                if actual == &key_with_colon {
                    YAMLBlockStartLineType::Angle
                } else {
                    YAMLBlockStartLineType::None
                }
            }
            _ => YAMLBlockStartLineType::None,
        }
    };
    // We skip everything until the first line that we expect, including that first line. We end up
    // either with an empty iterator or an iterator whose first element is the first value line.
    let mut skipped = text
        .lines()
        .skip_while(|line| !start_line_type(line).is_actual_start_line());
    let block_type = if let Some(line) = skipped.next() {
        start_line_type(line)
    } else {
        YAMLBlockStartLineType::None
    };
    let mut peekable = skipped.skip_while(|line| line.is_empty()).peekable();
    let first_line = peekable.peek();
    // Check whether we have a value line or not.
    if let Some(line) = first_line {
        // We check whether the first value line is indented. If so, we remember the indent since
        // every following value line has to have the exact same indent.
        let first_indent = line.len() - line.trim_start().len();
        if first_indent > 0 {
            let result = peekable
                .take_while(|line| {
                    line.is_empty() || line.len() - line.trim_start().len() == first_indent
                })
                .map(|line| line.trim())
                .collect::<Vec<&str>>()
                .join("\n");
            log::info!("found value for key {} from yaml:\n{}", key, result);
            match block_type {
                YAMLBlockStartLineType::Pipe => result,
                YAMLBlockStartLineType::Angle => result
                    .split("\n\n")
                    .map(|line| line.replace("\n", " "))
                    .collect::<Vec<_>>()
                    .join("\n"),
                YAMLBlockStartLineType::None => String::new(),
            }
        } else {
            log::info!("no value line found");
            String::new()
        }
    } else {
        log::info!("key {} not found", key);
        String::new()
    }
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

    #[test]
    fn applying_to_no_block_quotes_remains_unchanged() {
        let text = r#"
## Some Heading

Some text without block quotes.

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

        let unchanged = BlockQuotes::new(text).apply_to_matches_and_join(|_, _| String::new());
        assert_eq!(text.to_string(), unchanged);
    }

    #[test]
    fn applying_to_block_quotes() {
        let text = r#"
## Some Heading

Some text with block quotes.

> This first text is block quoted.
>
>> This text is quoted at the second level.
>
> Some more quotes at the first level.

<!-- some html -->

- More text.
- More text.
  - Even more text.
  - Some text with a [link].

> This second text is also block quoted.
>
> > This text is quoted at the second level.
>
> Some more quotes at the first level.

- Some text.

  > This third text is block quoted but inside an itemization.
  >
  >> This text is quoted at the second level.
  >
  > Some more quotes at the first level.

  More text.

[link]: https://something.com "some link"
"#;

        let expected = r#"
## Some Heading

Some text with block quotes.

> 2:115
> 2:115
> 2:115

<!-- some html -->

- More text.
- More text.
  - Even more text.
  - Some text with a [link].

> 2:121
> 2:121
> 2:121

- Some text.

  > 4:141
  > 4:141
  > 4:141

  More text.

[link]: https://something.com "some link"
"#;

        let changed = BlockQuotes::new(text).apply_to_matches_and_join(|s, i| {
            format!("{}:{}\n{}:{}\n{}:{}\n", i, s.len(), i, s.len(), i, s.len())
        });
        assert_eq!(expected, changed);
    }

    #[test]
    fn flattening_vecs_of_char_ranges_retains_ranges() {
        let to_be_flattened = vec![
            vec![CharRange { start: 0, end: 10 }],
            vec![
                CharRange {
                    start: 100,
                    end: 110,
                },
                CharRange {
                    start: 200,
                    end: 210,
                },
            ],
        ];
        let flat = to_be_flattened.into_iter().flatten().collect::<Vec<_>>();
        let expected = vec![(0..10), (100..110), (200..210)];
        assert_eq!(expected, flat);
    }

    fn build_yaml(
        key: &str,
        space_before_colon: bool,
        block_marker: &str,
        indent_spaces: usize,
        content: &str,
    ) -> String {
        let indent = (0..indent_spaces).map(|_| " ").collect::<String>();
        let indented = content
            .lines()
            .map(|line| format!("{}{}\n", indent, line))
            .collect::<String>();
        let maybe_space = if space_before_colon { " " } else { "" };
        let result = format!("{}{}: {}\n{}", key, maybe_space, block_marker, indented);
        // Ensure that values were filled in.
        assert_ne!(result, String::from(": \n"));
        result
    }

    const YAML_BASE_CONTENT: &str = r#"
some content with an empty line

at the beginning and in the middle"#;

    #[test]
    fn building_yaml() {
        let yaml = build_yaml(YAML_CONFIG_KEY, true, "|", 4, YAML_BASE_CONTENT);
        let expected = r#"mdslw-toml : |
    
    some content with an empty line
    
    at the beginning and in the middle
"#;
        assert_eq!(yaml, expected);
    }

    #[test]
    fn extracting_yaml_string_pipe_block_markers() {
        for has_space in [true, false] {
            for marker in ["|", "|-", "|+"] {
                let yaml = build_yaml(YAML_CONFIG_KEY, has_space, marker, 4, YAML_BASE_CONTENT);
                let extracted = get_value_for_mdslw_toml_yaml_key(&yaml);
                assert_eq!(extracted, YAML_BASE_CONTENT);
            }
        }
    }

    #[test]
    fn extracting_yaml_string_angle_block_markers() {
        let expected = r#" some content with an empty line
at the beginning and in the middle"#;
        for has_space in [true, false] {
            for marker in [">", ">-", ">+"] {
                let yaml = build_yaml(YAML_CONFIG_KEY, has_space, marker, 4, YAML_BASE_CONTENT);
                let extracted = get_value_for_mdslw_toml_yaml_key(&yaml);
                assert_eq!(extracted, expected);
            }
        }
    }

    #[test]
    fn extracting_yaml_string_pipe_block_markers_wrong_key() {
        let key = "some-other-key";
        assert_ne!(key, YAML_CONFIG_KEY);
        for has_space in [true, false] {
            for marker in ["|", "|-", "|+"] {
                let yaml = build_yaml(key, has_space, marker, 4, YAML_BASE_CONTENT);
                let extracted = get_value_for_mdslw_toml_yaml_key(&yaml);
                assert_eq!(extracted, String::new());
            }
        }
    }

    #[test]
    fn extracting_yaml_string_angle_block_markers_wrong_key() {
        let key = "some-other-key";
        assert_ne!(key, YAML_CONFIG_KEY);
        for has_space in [true, false] {
            for marker in [">", ">-", ">+"] {
                let yaml = build_yaml(key, has_space, marker, 4, YAML_BASE_CONTENT);
                let extracted = get_value_for_mdslw_toml_yaml_key(&yaml);
                assert_eq!(extracted, String::new());
            }
        }
    }

    #[test]
    fn extracting_yaml_string_empty_content() {
        let key = "some-other-key";
        for has_space in [true, false] {
            for marker in ["|", "|-", "|+"] {
                let yaml = build_yaml(YAML_CONFIG_KEY, has_space, marker, 4, "")
                    + build_yaml(key, has_space, marker, 4, "").as_str();
                let extracted = get_value_for_mdslw_toml_yaml_key(&yaml);
                assert_eq!(extracted, "");
            }
        }
    }
}
