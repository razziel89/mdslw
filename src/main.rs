mod indent;
mod linebreak;
mod parse;

use crate::indent::{fill_ranges, spaces, TextRange};
use crate::linebreak::insert_linebreaks_between_sentences;
use crate::parse::parse;

fn read_stdin() -> String {
    std::io::stdin()
        .lines()
        // Ignore lines that cannot be read.
        .filter_map(|el| el.ok())
        .collect::<Vec<_>>()
        .join("\n")
}

fn format(text: &String, ranges: Vec<TextRange>) -> String {
    let mut result = String::new();

    for range in ranges {
        if range.verbatim {
            result.push_str(&text[range.range]);
        } else {
            let indent = spaces(range.indent_spaces);
            result.push_str(&insert_linebreaks_between_sentences(
                &text[range.range],
                &indent,
            ));
        }
    }

    result
}

fn main() {
    let markdown = read_stdin();
    let parsed = parse(&markdown);
    let filled = fill_ranges(parsed, &markdown);

    println!("{}", format(&markdown, filled));
}
