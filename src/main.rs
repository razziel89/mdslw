use core::ops::Range;
use pulldown_cmark::{Event, Parser, Tag};
use std::collections::HashMap;

const SPACES_PER_TAB: usize = 4;

fn read_stdin() -> String {
    std::io::stdin()
        .lines()
        // Ignore lines that cannot be read.
        .filter_map(|el| el.ok())
        .collect::<Vec<_>>()
        .join("\n")
}

type TextRange = Range<usize>;

// Filter out those ranges of text that shall be wrapped.
fn to_be_wrapped(events: Vec<(Event, TextRange)>) -> Vec<TextRange> {
    let mut verbatim_level: usize = 0;

    events
        .into_iter()
        .filter(|(event, _range)| match event {
            Event::Start(tag) => {
                // println!("START {:?} {:?}", tag, _range);
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
                // println!("END {:?} {:?}", tag, _range);
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
            Event::Html(..)
            | Event::TaskListMarker(..)
            | Event::FootnoteReference(..)
            | Event::Rule => false,

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

// Get all indices that point to whitespace as well as the characters they point to.
fn whitespace_indices(text: &String) -> HashMap<usize, char> {
    text.char_indices()
        .filter_map(|(pos, ch)| {
            if ch.is_whitespace() {
                Some((pos, ch))
            } else {
                None
            }
        })
        .collect::<HashMap<_, _>>()
}

fn merge_ranges(ranges: Vec<TextRange>, whitespaces: HashMap<usize, char>) -> Vec<TextRange> {
    let mut next_range: Option<TextRange> = None;
    let mut merged = vec![];

    for range in ranges {
        if let Some(next) = next_range {
            // Check whether there is nothing but whitespace between the end of the previous range
            // and the start of the next one, if the ranges do not connect directly anyway. Note
            // that we still keep paragraphs separated by keeping ranges separate that are
            // separated by more than one linebreak.
            let contains_just_whitespace =
                (next.end..range.start).all(|el| whitespaces.contains_key(&el));
            let at_most_one_linebreak = (next.end..range.start)
                .filter(|el| Some(&'\n') == whitespaces.get(&el))
                .count()
                <= 1;

            if contains_just_whitespace && at_most_one_linebreak {
                // Extend the range.
                next_range = Some(TextRange {
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

fn ignore_for_indent(ch: &char) -> bool {
    ch == &'-' || ch == &'*' || ch.is_whitespace()
}

fn extract_indent(line: &str) -> String {
    line.chars()
        .take_while(|el| ignore_for_indent(el))
        .map(|_| ' ')
        .collect::<String>()
}

// Get indent used for each line. This replaces tabs by a certain number of spaces.
fn indent_for_line(text: &String, spaces_per_tab: usize) -> HashMap<usize, String> {
    let spaces = (0..spaces_per_tab).map(|_| " ").collect::<String>();

    text.replace("	", &spaces)
        .split("\n")
        .enumerate()
        .map(|(line_nr, line)| (line_nr, extract_indent(line)))
        .collect::<HashMap<_, _>>()
}

fn linebreak_positions(text: &String) -> Vec<usize> {
    text.chars()
        .enumerate()
        .filter_map(|(pos, ch)| if ch == '\n' { Some(pos) } else { None })
        .collect::<Vec<_>>()
}

fn parse(text: &String) -> Vec<TextRange> {
    let events_and_ranges = Parser::new(&text).into_offset_iter().collect::<Vec<_>>();
    let whitespaces = whitespace_indices(&text);

    merge_ranges(to_be_wrapped(events_and_ranges), whitespaces)
}

// fn wrap(text: &String, wrap_ranges: Vec<TextRange>) -> Vec<String> {
//     let linebreaks = linebreak_positions(&text);
//     let indents = indent_for_line(&text, SPACES_PER_TAB);
//
//     vec![]
// }

fn format(text: &String, ranges: Vec<TextRange>) -> String {
    let mut result = String::new();

    for range in ranges {
        result.push_str("===='");
        result.push_str(&text[range]);
        result.push_str("'====\n\n");
    }

    result
}

fn main() {
    let markdown = read_stdin();
    let parsed = parse(&markdown);

    println!("{}", format(&markdown, parsed));
}
