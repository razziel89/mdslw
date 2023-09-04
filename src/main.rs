mod parse;

use std::collections::HashMap;

use crate::parse::{parse, TextRange};

const SPACES_PER_TAB: usize = 4;

fn read_stdin() -> String {
    std::io::stdin()
        .lines()
        // Ignore lines that cannot be read.
        .filter_map(|el| el.ok())
        .collect::<Vec<_>>()
        .join("\n")
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
