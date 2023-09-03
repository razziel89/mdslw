use core::ops::Range;
use pulldown_cmark::{Event, Parser, Tag};

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
                println!("START {:?} {:?}", tag, _range);
                match tag {
                    // Most other delimited blocks should stay as they are.
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
                    // Other delimited blocks can be both, inside a verbatim block or inside text.
                    // However, the text they embrace is the important bit but we do not want to
                    // extract the entire range.
                    Tag::Emphasis
                    | Tag::Item
                    | Tag::Link(..)
                    | Tag::List(..)
                    | Tag::Paragraph
                    | Tag::Strikethrough
                    | Tag::Strong => false,
                }
            }

            Event::End(tag) => {
                println!("END {:?} {:?}", tag, _range);
                match tag {
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

                    Tag::Emphasis
                    | Tag::Item
                    | Tag::Link(..)
                    | Tag::List(..)
                    | Tag::Paragraph
                    | Tag::Strikethrough
                    | Tag::Strong => false,
                }
            }

            // More elements that should be taken verbatim but that are not blocks.
            Event::Html(..)
            | Event::TaskListMarker(..)
            | Event::FootnoteReference(..)
            | Event::Rule => false,

            // The following should be wrapped if they are not inside a verbatim block.
            Event::SoftBreak | Event::HardBreak | Event::Text(..) | Event::Code(..) => {
                verbatim_level == 0
            }
        })
        .map(|(_event, range)| range)
        .collect::<Vec<_>>()
}

fn parse(text: &String) -> Vec<TextRange> {
    let parser = Parser::new(&text);
    let iterator = parser.into_offset_iter();

    to_be_wrapped(iterator.into_iter().collect::<Vec<_>>())
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
