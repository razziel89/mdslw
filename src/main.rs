use core::ops::Range;
use pulldown_cmark::{Event, Parser, Tag};
use std::collections::HashSet;

fn read_stdin() -> String {
    std::io::stdin()
        .lines()
        // Ignore lines that cannot be read.
        .filter_map(|el| el.ok())
        .collect::<Vec<_>>()
        .join("\n")
}

type TextRange = Range<usize>;

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

fn whitespace_indices(text: &String) -> HashSet<usize> {
    text.char_indices()
        .filter_map(
            |(pos, ch)| {
                if ch.is_whitespace() {
                    Some(pos)
                } else {
                    None
                }
            },
        )
        .collect::<HashSet<_>>()
}

fn merge_ranges(ranges: Vec<TextRange>, whitespaces: HashSet<usize>) -> Vec<TextRange> {
    let mut next_range: Option<TextRange> = None;
    let mut merged = vec![];

    for range in ranges {
        if let Some(next) = next_range {
            if next.end == range.start {
                next_range = Some(TextRange {
                    start: next.start,
                    end: range.end,
                });
            } else {
                merged.push(next);
                next_range = Some(range);
            }
        } else {
            next_range = Some(range);
        }
    }

    if let Some(next) = next_range {
        merged.push(next)
    }

    // Remove ranges that do not contain at least 1 character. They never have to be wrapped.
    merged
        .into_iter()
        .filter(|el| el.len() > 1)
        .collect::<Vec<_>>()
}

fn parse(text: &String) -> Vec<TextRange> {
    let events_and_ranges = Parser::new(&text).into_offset_iter().collect::<Vec<_>>();
    let whitespaces = whitespace_indices(&text);

    merge_ranges(to_be_wrapped(events_and_ranges), whitespaces)
}

fn format(text: &String, ranges: Vec<TextRange>) -> String {
    let mut result = String::new();

    for range in ranges {
        result.push_str("'");
        result.push_str(&text[range]);
        result.push_str("'\n");
    }

    result
}

fn main() {
    let markdown = read_stdin();
    let parsed = parse(&markdown);

    println!("{}", format(&markdown, parsed));
}
