mod indent;
mod parse;

use std::collections::HashSet;

use crate::indent::{fill_ranges, spaces, TextRange};
use crate::parse::parse;

fn read_stdin() -> String {
    std::io::stdin()
        .lines()
        // Ignore lines that cannot be read.
        .filter_map(|el| el.ok())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Replace all consecutive whitespace by a single space. That includes line breaks. This is like
/// piping through `tr -s '[:space:]' ' '` in the shell.
fn merge_all_whitespace(text: &str) -> String {
    let mut last_was_whitespace = false;

    text.chars()
        .filter_map(|el| {
            if el.is_whitespace() {
                if last_was_whitespace {
                    None
                } else {
                    last_was_whitespace = true;
                    Some(' ')
                }
            } else {
                last_was_whitespace = false;
                Some(el)
            }
        })
        .collect::<String>()
}

// fn find_sentence_ends(text: &str) -> HashSet<usize> {
//     let potential_positions = text.find(pat)
//
// }

fn is_sentence_end(ch: char) -> bool {
    ch == '.' || ch == '!' || ch == '?'
}

fn insert_linebreaks_between_sentences(text: &str, indent: &str) -> String {
    merge_all_whitespace(text)
        .split_inclusive(is_sentence_end)
        .enumerate()
        .map(|(_idx, el)| {
            if el.ends_with(is_sentence_end) {
                format!("{}\n{}", el.trim_start(), indent)
            } else {
                format!("{}", el)
            }
        })
        .collect::<Vec<_>>()
        .join("")
        .trim_end()
        .to_string()
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
