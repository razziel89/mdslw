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
use std::fmt::Write;
use std::iter::repeat;

use crate::detect::WhitespaceDetector;
use crate::trace_log;

const DEFAULT_CATEGORY: &str = "DEFAULT UNDEFINED CATEGORY";

#[derive(Clone, PartialEq)]
enum CharEnv {
    LinkInRange,
    NonLinkInRange,
    LinkDef,
}

#[derive(Hash, Eq, PartialEq, Debug)]
enum LineType<'a> {
    Empty,
    LinkDef,
    LinkCategory(&'a str),
    Other,
}

pub fn replace_spaces_in_links_by_nbsp(text: String) -> String {
    let text_no_nbsp = text
        .chars()
        .map(|ch| {
            if !ch.is_whitespace() || ch.is_ascii_whitespace() {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>();

    // First, determine all byte positions that the parser recognised.
    let mut byte_indices_in_links = Parser::new(&text_no_nbsp)
        .into_offset_iter()
        .flat_map(|(_event, range)| range.zip(repeat(CharEnv::NonLinkInRange)))
        .collect::<HashMap<_, _>>();

    // Then, determine all byte positions in links. We cannot use the "_ =>" branch below because
    // ranges overlap and the link ranges will be undone by the wrapping ranges.
    byte_indices_in_links.extend(
        Parser::new(&text_no_nbsp)
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
    let byte_indices_in_link_defs = text_no_nbsp
        .split_inclusive('\n')
        .filter_map(|line| {
            let start = line_start;
            line_start += line.len();
            // Only process lines outside of ranges that start with an open bracket.
            if line.starts_with('[') && !byte_indices_in_links.contains_key(&start) {
                line.find("]:")
                    .map(|close| (start..start + close).zip(repeat(CharEnv::LinkDef)))
            } else {
                None
            }
        })
        .flatten()
        .collect::<HashMap<_, _>>();

    byte_indices_in_links.extend(byte_indices_in_link_defs);

    let mut last_replaced = false;
    text.chars()
        .zip(text_no_nbsp.char_indices())
        .filter_map(|(ch, (idx, _ch))| {
            let ch_env = byte_indices_in_links.get(&idx);
            if ch.is_whitespace()
                && (ch_env == Some(&CharEnv::LinkInRange) || ch_env == Some(&CharEnv::LinkDef))
            {
                if last_replaced {
                    None
                } else {
                    last_replaced = true;
                    Some('\u{a0}')
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

    // Then determine the type of each line. We will rearrange lines when collating.
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
            } else if let Some(category) =
                // We are trying to extract the link category from the line. This is how we do it.
                line
                    .trim_end_matches('\n')
                    .strip_prefix("<!--")
                    .and_then(|el| el.strip_suffix("-->"))
                    .map(str::trim)
                    .and_then(|el| el.strip_prefix("link-category:"))
            {
                // This nested if will become obsolete once let-chains have been stabilised.
                // We accept all link category names that do not end the HTML comment.
                if !category.contains("-->") {
                    LineType::LinkCategory(category.trim())
                } else {
                    LineType::Other
                }
            } else {
                LineType::Other
            }
        })
        .collect::<Vec<_>>();

    // We treat user-defined catgories and the default category differently. That is, we always
    // keep user-defined ones but output the default one only if it contains at least one link def.
    let user_defined_categories = line_types
        .iter()
        .filter_map(|t| {
            if let LineType::LinkCategory(cat) = t {
                if cat != &DEFAULT_CATEGORY {
                    Some(cat)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<HashSet<_>>();
    let mut user_defined_categories = user_defined_categories.into_iter().collect::<Vec<_>>();
    user_defined_categories.sort_by_key(|s| s.to_lowercase());

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
        "found {} lines with user-defined link category definitions: {:?}",
        user_defined_categories.len(),
        user_defined_categories
    );
    trace_log!(
        "found {} lines of none of the other types",
        line_types.iter().filter(|t| t == &&LineType::Other).count()
    );

    let mut last_output_line_is_empty = true;
    let resulting_text = text
        .split_inclusive('\n')
        .enumerate()
        .filter_map(|(idx, line)| {
            let this_type = line_types.get(idx);
            let next_type = line_types.get(idx + 1);

            if this_type == Some(&LineType::Other)
                || (this_type == Some(&LineType::Empty)
                    && next_type != Some(&LineType::LinkDef)
                    && !matches!(next_type, Some(&LineType::LinkCategory(_))))
            {
                last_output_line_is_empty = this_type == Some(&LineType::Empty);
                Some(line)
            } else {
                None
            }
        })
        .collect::<String>();

    let mut current_category = &DEFAULT_CATEGORY;
    let mut categories_and_links = text
        .split_inclusive('\n')
        .enumerate()
        .filter_map(|(idx, line)| {
            let this_type = line_types.get(idx).unwrap_or(&LineType::Other);
            if let LineType::LinkCategory(cat) = this_type {
                current_category = cat;
            }
            if this_type == &LineType::LinkDef {
                if line.ends_with('\n') {
                    Some((current_category, line.to_owned()))
                } else {
                    Some((current_category, line.to_owned() + "\n"))
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    categories_and_links.sort_by_key(|(_cat, link_def)| link_def.to_lowercase());

    // Check whether we have to add a number of newline characters to make sure that the block of
    // links at the end is separated by an empty line.
    let whitespace_to_add = match (
        categories_and_links.is_empty(),
        resulting_text.is_empty(),
        last_output_line_is_empty,
        resulting_text.ends_with('\n'),
    ) {
        // There are no link defs. Add none.
        (true, _, _, _) => "",
        // There is no text. Add none.
        (_, true, _, _) => "",
        // There are link defs and there is text.
        // -> No empty line at end and the text does not end in a newline. Add two.
        (false, false, false, false) => "\n\n",
        // -> No empty line at end but the text does end in a newline. Add one.
        (false, false, false, true) => "\n",
        // -> An empty line at end but it does not end in a newline. Add one.
        (false, false, true, false) => "\n",
        // -> An empty line at end and it does end in a newline. Add none.
        (false, false, true, true) => "",
    };

    let link_defs_block = if user_defined_categories.is_empty() {
        log::debug!("there are no user-defined categories, simply sorting link defs");
        categories_and_links
            .into_iter()
            .map(|(_category, link)| link)
            .collect::<String>()
    } else {
        log::debug!("there are user-defined categories, sorting link defs by category");

        // The "write!" calls below never fail since we write to a String that we create here.
        let mut block = String::new();

        // We always write out all user-defined categories even if they are empty.
        // Nested for loops are not efficient, but it's OK in this case.
        let mut last_category_had_entries = false;
        for cat in user_defined_categories {
            log::debug!("processing user-defined category: {}", cat);
            let white_space = if last_category_had_entries { "\n" } else { "" };
            writeln!(block, "{}<!-- link-category: {} -->\n", white_space, cat)
                .expect("building block of link categories");
            last_category_had_entries = false;
            for (category, link_def) in categories_and_links.iter() {
                if category == &cat {
                    last_category_had_entries = true;
                    log::debug!("found link def in category: {}", link_def.trim());
                    write!(block, "{}", link_def).expect("building block of link categories");
                }
            }
        }

        // We only write out the default category if it contains link defs.
        let links_in_default_category = categories_and_links
            .into_iter()
            .filter_map(|(category, link_def)| {
                if category == &DEFAULT_CATEGORY {
                    Some(link_def)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if !links_in_default_category.is_empty() {
            log::debug!("processing default category: {}", DEFAULT_CATEGORY);
            links_in_default_category
                .iter()
                .for_each(|el| log::debug!("found link def in default category: {}", el.trim()));
            let white_space = if last_category_had_entries { "\n" } else { "" };
            write!(
                block,
                "{}<!-- link-category: {} -->\n\n{}",
                white_space,
                DEFAULT_CATEGORY,
                links_in_default_category.join("")
            )
            .expect("building block of link categories");
        }

        block
    };

    format!("{}{}{}", resulting_text, whitespace_to_add, link_defs_block)
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
        // Broken links, i.e. links whose target cannot be found, e.g. because of something other
        // than a mismatch of non-breaking spaces, will not be recognised as links by the parser
        // and, thus, do not have their spaces adjusted.
        let original = "\
            [broken\u{a0}link with a\u{a0}few nbsp]\n\n\
            [named broken\u{a0}link with a\u{a0}few nbsp][named link]\n\n\
            [link\u{a0}with\u{a0}a\u{a0}few\u{a0}nbsp]: http://some-link\n\
            [differently\u{a0}named\u{a0}link]: http://other-link\n\
            ";

        let replaced = replace_spaces_in_links_by_nbsp(original.to_string());

        assert_eq!(replaced, original);
    }

    #[test]
    fn replacing_spaces_for_broken_links_due_to_nbsp() {
        let original = "\
            [broken\u{a0}link with a\u{a0}few nbsp]\n\n\
            [named broken\u{a0}link with a\u{a0}few nbsp][named link]\n\n\
            [broken\u{a0}link\u{a0}with\u{a0}a\u{a0}few\u{a0}nbsp]: http://some-link\n\
            [named\u{a0}link]: http://other-link\n\
            ";
        let expected = "\
            [broken\u{a0}link\u{a0}with\u{a0}a\u{a0}few\u{a0}nbsp]\n\n\
            [named\u{a0}broken\u{a0}link\u{a0}with\u{a0}a\u{a0}few\u{a0}nbsp][named\u{a0}link]\n\n\
            [broken\u{a0}link\u{a0}with\u{a0}a\u{a0}few\u{a0}nbsp]: http://some-link\n\
            [named\u{a0}link]: http://other-link\n\
            ";

        let replaced = replace_spaces_in_links_by_nbsp(original.to_string());

        assert_eq!(replaced, expected);
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

    #[test]
    fn creating_empty_lines_if_needed() {
        let original_1 = "\
            [link ref]: http://some-link\n\n\
            [link ref]\
            ";
        let original_2 = "\
            [link ref]: http://some-link\n\n\
            [link ref]\n\
            ";
        let expected = "\
            \n[link ref]\n\n\
            [link ref]: http://some-link\n\
            ";

        let collated_1 =
            collate_link_defs_at_end(original_1.to_string(), &WhitespaceDetector::new(false));
        assert_eq!(collated_1, expected);

        let collated_2 =
            collate_link_defs_at_end(original_2.to_string(), &WhitespaceDetector::new(false));
        assert_eq!(collated_2, expected);
    }

    #[test]
    fn categorising_and_sorting_link_defs() {
        let original = "\
            [link ref]\n\n\
            [another link ref]\n\n\
            <!-- link-category: zzz -->\n\n\
            [named link]: http://other-link\n\
            [another named link]: http://yet-another-link\n\n\
            <!-- link-category: asdf -->\n\n\
            [link ref]: http://some-link\n\
            [another link ref]: http://another-link\n\n\
            [named link ref][named link]\n\n\
            [another named link ref][another named link]\n\
            ";
        let expected = "\
            [link ref]\n\n\
            [another link ref]\n\n\
            [named link ref][named link]\n\n\
            [another named link ref][another named link]\n\n\
            <!-- link-category: asdf -->\n\n\
            [another link ref]: http://another-link\n\
            [link ref]: http://some-link\n\n\
            <!-- link-category: zzz -->\n\n\
            [another named link]: http://yet-another-link\n\
            [named link]: http://other-link\n\
            ";

        let collated =
            collate_link_defs_at_end(original.to_string(), &WhitespaceDetector::new(false));

        assert_eq!(collated, expected);
    }

    #[test]
    fn using_default_category_for_uncategorised_links() {
        let original = "\
            [link ref]\n\n\
            [another link ref]\n\n\
            [named link]: http://other-link\n\
            [another named link]: http://yet-another-link\n\n\
            <!-- link-category: asdf -->\n\n\
            [link ref]: http://some-link\n\
            [another link ref]: http://another-link\n\n\
            [named link ref][named link]\n\n\
            [another named link ref][another named link]\n\
            ";
        let expected = "\
            [link ref]\n\n\
            [another link ref]\n\n\
            [named link ref][named link]\n\n\
            [another named link ref][another named link]\n\n\
            <!-- link-category: asdf -->\n\n\
            [another link ref]: http://another-link\n\
            [link ref]: http://some-link\n\n\
            <!-- link-category: DEFAULT UNDEFINED CATEGORY -->\n\n\
            [another named link]: http://yet-another-link\n\
            [named link]: http://other-link\n\
            ";

        let collated =
            collate_link_defs_at_end(original.to_string(), &WhitespaceDetector::new(false));

        assert_eq!(collated, expected);
    }

    #[test]
    fn keeping_empty_user_defined_categories_but_not_empty_default_one() {
        let original = "\
            [named link ref][named link]\n\n\
            [another named link ref][another named link]\n\n\
            <!-- link-category: zzz -->\n\n\
            [another named link]: http://yet-another-link\n\
            [named link]: http://other-link\n\n\
            <!-- link-category: DEFAULT UNDEFINED CATEGORY -->\n\n\
            <!-- link-category: asdf -->\n\
            ";
        let expected = "\
            [named link ref][named link]\n\n\
            [another named link ref][another named link]\n\n\
            <!-- link-category: asdf -->\n\n\
            <!-- link-category: zzz -->\n\n\
            [another named link]: http://yet-another-link\n\
            [named link]: http://other-link\n\
            ";

        let collated =
            collate_link_defs_at_end(original.to_string(), &WhitespaceDetector::new(false));

        assert_eq!(collated, expected);
    }
}
