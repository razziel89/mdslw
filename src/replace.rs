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

use pulldown_cmark::{Event, Parser, Tag};
use std::collections::{HashMap, HashSet};
use std::iter::repeat;

use crate::detect::WhitespaceDetector;
use crate::trace_log;

#[derive(Clone, PartialEq)]
enum CharEnv {
    LinkInRange,
    NonLinkInRange,
    LinkDef,
}

#[derive(Hash, Eq, PartialEq)]
enum LineType {
    Empty,
    LinkDef,
    Other,
}

pub fn replace_spaces_in_links_by_nbsp(text: String) -> String {
    // First, determine all byte positions that the parser recognised.
    let mut char_indices_in_links = Parser::new(&text)
        .into_offset_iter()
        .flat_map(|(_event, range)| range.zip(repeat(CharEnv::NonLinkInRange)))
        .collect::<HashMap<_, _>>();

    // Then, determine all byte positions in links. We cannot use the "_ =>" branch below because
    // ranges overlap and the link ranges will be undone by the wrapping ranges.
    char_indices_in_links.extend(
        Parser::new(&text)
            .into_offset_iter()
            .filter_map(|(event, range)| match event {
                Event::Start(Tag::Link { .. }) => Some(range.zip(repeat(CharEnv::LinkInRange))),
                _ => None,
            })
            .flatten(),
    );

    // Then, determine all byte positions in link definitions. The parser completely ignores such
    // lines, which means we have to detect them manually. We do so by only looking at lines that
    // the parser ignored and then filtering for lines that contain the `[some text]:` syntax,
    // which indicates link definitions. We then allow replacing all the lines in the link text.
    let mut line_start = 0;
    let char_indices_in_link_defs = text
        .split_inclusive('\n')
        .filter_map(|line| {
            let start = line_start;
            line_start += line.len();
            // Only process lines outside of ranges that start with an open bracket.
            if line.starts_with('[') && !char_indices_in_links.contains_key(&start) {
                line.find("]:")
                    .map(|close| (start..start + close).zip(repeat(CharEnv::LinkDef)))
            } else {
                None
            }
        })
        .flatten()
        .collect::<HashMap<_, _>>();

    char_indices_in_links.extend(char_indices_in_link_defs);

    let mut last_replaced = false;
    text.char_indices()
        .filter_map(|(idx, ch)| {
            let ch_env = char_indices_in_links.get(&idx);
            if ch.is_whitespace()
                && (ch_env == Some(&CharEnv::LinkInRange) || ch_env == Some(&CharEnv::LinkDef))
            {
                if last_replaced {
                    None
                } else {
                    last_replaced = true;
                    Some('\u{00a0}')
                }
            } else {
                last_replaced = false;
                Some(ch)
            }
        })
        .collect::<String>()
}

pub fn collate_link_defs_at_end(text: String, detector: &WhitespaceDetector) -> String {
    // First, determine all byte positions that the parser recognised.
    let char_indices_recognised_by_parser = Parser::new(&text)
        .into_offset_iter()
        .flat_map(|(_event, range)| range)
        .collect::<HashSet<_>>();

    let mut line_start = 0;
    let line_types = text
        .split_inclusive('\n')
        .map(|line| {
            let start = line_start;
            line_start += line.len();
            if line.chars().all(|ch| detector.is_whitespace(&ch)) {
                LineType::Empty
            } else if line.starts_with('[')
                && !char_indices_recognised_by_parser.contains(&start)
                && line.contains("]:")
            {
                LineType::LinkDef
            } else {
                LineType::Other
            }
        })
        .collect::<Vec<_>>();

    trace_log!(
        "found {} empty lines",
        line_types.iter().filter(|t| t == &&LineType::Empty).count()
    );
    trace_log!(
        "found {} lines with link definitions",
        line_types
            .iter()
            .filter(|t| t == &&LineType::LinkDef)
            .count()
    );
    trace_log!(
        "found {} lines from neither category",
        line_types.iter().filter(|t| t == &&LineType::Other).count()
    );

    let mut last_output_line_is_empty = true;
    let result = text
        .split_inclusive('\n')
        .enumerate()
        .filter_map(|(idx, line)| {
            let this_type = line_types.get(idx);
            let next_type = line_types.get(idx + 1);

            if this_type == Some(&LineType::Other)
                || (this_type == Some(&LineType::Empty) && next_type != Some(&LineType::LinkDef))
            {
                last_output_line_is_empty = this_type == Some(&LineType::Empty);
                Some(line)
            } else {
                None
            }
        })
        .collect::<String>();

    let mut links = text
        .split_inclusive('\n')
        .enumerate()
        .filter_map(|(idx, line)| {
            let this_type = line_types.get(idx).unwrap_or(&LineType::Other);
            if this_type == &LineType::LinkDef {
                if line.ends_with('\n') {
                    Some(line.to_owned())
                } else {
                    Some(line.to_owned() + "\n")
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    links.sort_by_key(|s| s.to_lowercase());

    // If there are links to be collated and the current text does not already end in an empty
    // line, we will add one.
    let break_to_add = if !links.is_empty() && !last_output_line_is_empty {
        "\n"
    } else {
        ""
    };

    format!("{}{}{}", result, break_to_add, links.join(""))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn replacing_spaces_only_in_links() {
        let original = "Outside of link, [inside of link](http://some-url), again outside.";
        let expected =
            "Outside of link, [inside\u{a0}of\u{a0}link](http://some-url), again outside.";

        let replaced = replace_spaces_in_links_by_nbsp(original.to_string());

        assert_eq!(replaced, expected);
    }

    #[test]
    fn replacing_all_spaces_in_links_even_if_there_are_some_nbsp() {
        let original = "Some initial text, [link\u{a0}with some\u{a0}nbsp](http://some-url)";
        let expected = "Some initial text, [link\u{a0}with\u{a0}some\u{a0}nbsp](http://some-url)";

        let replaced = replace_spaces_in_links_by_nbsp(original.to_string());

        assert_eq!(replaced, expected);
    }

    #[test]
    fn replacing_spaces_also_in_link_defs() {
        let original = "\
            [link ref]\n\n\
            [named link ref][named link]\n\n\
            [link ref]: http://some-link\n\
            [named link]: http://other-link\n\
            ";
        let expected = "\
            [link\u{a0}ref]\n\n\
            [named\u{a0}link\u{a0}ref][named\u{a0}link]\n\n\
            [link\u{a0}ref]: http://some-link\n\
            [named\u{a0}link]: http://other-link\n\
            ";

        let replaced = replace_spaces_in_links_by_nbsp(original.to_string());

        assert_eq!(replaced, expected);
    }

    #[test]
    fn replacing_all_spaces_in_link_defs_even_if_there_are_some_nbsp() {
        let original = "\
            [link with a\u{a0}few nbsp]\n\n\
            [named link with a\u{a0}few nbsp][named link]\n\n\
            [link with a\u{a0}few nbsp]: http://some-link\n\
            [named link]: http://other-link\n\
            ";
        let expected = "\
            [link\u{a0}with\u{a0}a\u{a0}few\u{a0}nbsp]\n\n\
            [named\u{a0}link\u{a0}with\u{a0}a\u{a0}few\u{a0}nbsp][named\u{a0}link]\n\n\
            [link\u{a0}with\u{a0}a\u{a0}few\u{a0}nbsp]: http://some-link\n\
            [named\u{a0}link]: http://other-link\n\
            ";

        let replaced = replace_spaces_in_links_by_nbsp(original.to_string());

        assert_eq!(replaced, expected);
    }

    #[test]
    fn not_replacing_spaces_for_broken_links() {
        // Broken links, i.e. links whose target cannot be found, e.g. because of a mismatch of
        // non-breaking spaces, will not be recognised as links by the parser and, thus, do not
        // have their spaces adjusted. Note how there is a mismatch in non-breaking spaces between
        // the references in the first two lines and the link definitions in the last two lines.
        // Only the link definitions, since they are complete, would have their spaces adjusted.
        let original = "\
            [broken\u{a0}link with a\u{a0}few nbsp]\n\n\
            [named broken\u{a0}link with a\u{a0}few nbsp][named link]\n\n\
            [broken\u{a0}link\u{a0}with\u{a0}a\u{a0}few\u{a0}nbsp]: http://some-link\n\
            [named\u{a0}broken\u{a0}link\u{a0}with\u{a0}a\u{a0}few\u{a0}nbsp]: http://other-link\n\
            ";

        let replaced = replace_spaces_in_links_by_nbsp(original.to_string());

        assert_eq!(replaced, original);
    }

    #[test]
    fn colating_links_at_end_and_adding_an_empty_line_if_needed() {
        let original = "\
            [link ref]\n\n\
            [named link]: http://other-link\n\
            [link ref]: http://some-link\n\n\
            [named link ref][named link]\n\
            ";
        let expected = "\
            [link ref]\n\n\
            [named link ref][named link]\n\n\
            [link ref]: http://some-link\n\
            [named link]: http://other-link\n\
            ";

        let collated =
            collate_link_defs_at_end(original.to_string(), &WhitespaceDetector::new(false));

        assert_eq!(collated, expected);
    }

    #[test]
    fn keeping_empty_lines_at_end_when_there_are_no_links() {
        let original = "Some text.\n  \n \t \n";
        let expected = "Some text.\n  \n \t \n";

        let collated =
            collate_link_defs_at_end(original.to_string(), &WhitespaceDetector::new(false));

        assert_eq!(collated, expected);
    }

    #[test]
    fn keeping_link_only_documents_as_is() {
        let original = "\
            [named link]: http://other-link\n\
            [link ref]: http://some-link\n\
            [other link]: http://yet-another-link\n\
            ";
        let expected = "\
            [link ref]: http://some-link\n\
            [named link]: http://other-link\n\
            [other link]: http://yet-another-link\n\
            ";

        let collated =
            collate_link_defs_at_end(original.to_string(), &WhitespaceDetector::new(false));

        assert_eq!(collated, expected);
    }

    #[test]
    fn preserving_empty_lines_in_markdown_constructs() {
        let original = "\
            ```\n\n\n```\n\n\
            [link ref]: http://some-link\n\n\
            [link ref]\n\n\
            ";
        let expected = "\
            ```\n\n\n```\n\n\
            [link ref]\n\n\
            [link ref]: http://some-link\n\
            ";

        let collated =
            collate_link_defs_at_end(original.to_string(), &WhitespaceDetector::new(false));

        assert_eq!(collated, expected);
    }

    #[test]
    fn missing_newline_at_end_is_no_problem() {
        let original = "\
            [link ref]: http://some-link\n\
            [another link]: http://some-link\
            ";
        let expected = "\
            [another link]: http://some-link\n\
            [link ref]: http://some-link\n\
            ";

        let collated =
            collate_link_defs_at_end(original.to_string(), &WhitespaceDetector::new(false));

        assert_eq!(collated, expected);
    }
}
