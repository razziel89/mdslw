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

use std::collections::HashSet;

use crate::keep::KeepWords;

pub fn insert_linebreaks_between_sentences(
    text: &str,
    indent: &str,
    end_markers: &str,
    keep_words: &KeepWords,
) -> String {
    let merged = merge_all_whitespace(text);
    let sentence_ends = find_sentence_ends(&merged, end_markers, keep_words);

    merged
        .chars()
        .enumerate()
        .filter_map(|(idx, el)| {
            if sentence_ends.contains(&Char::Skip(idx)) {
                None
            } else if sentence_ends.contains(&Char::Split(idx)) {
                Some(format!("\n{}{}", indent, el))
            } else {
                Some(format!("{}", el))
            }
        })
        .collect::<String>()
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

#[derive(Eq, Hash, PartialEq, Debug)]
enum Char {
    Skip(usize),
    Split(usize),
}

fn find_sentence_ends(text: &str, end_markers: &str, keep_words: &KeepWords) -> HashSet<Char> {
    let as_chars = text.chars().collect::<Vec<_>>();

    text.chars()
        .zip(text.chars().skip(1))
        .enumerate()
        .filter_map(|(idx, (first, second))| {
            let keep_word = keep_words.ends_with_word(&as_chars, &idx);
            if !keep_word && second.is_whitespace() && end_markers.contains(first) {
                Some([Char::Skip(idx + 1), Char::Split(idx + 2)])
            } else {
                None
            }
        })
        .flatten()
        .collect::<HashSet<_>>()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn finding_sentence_ends() {
        let text = "words that. are. followed by. periods. period.";
        let keep = KeepWords::new("are. by.", "", false);
        let markers = ".";

        let ends = find_sentence_ends(text, markers, &keep);

        // We never detect a sentence at and the end of the text.
        let expected = vec![
            Char::Skip(11),
            Char::Split(12),
            Char::Skip(38),
            Char::Split(39),
        ]
        .into_iter()
        .collect::<HashSet<_>>();

        assert_eq!(expected, ends);
    }

    #[test]
    fn merging_whitespace() {
        // All whitespace, including tabs, is merged into single spaces.
        let text = " 	text with 	 lots of   white     space   	   ";
        let expected = " text with lots of white space ";

        let merged = merge_all_whitespace(text);

        assert_eq!(expected, merged);
    }

    #[test]
    fn inserting_linebreaks_between_sentences() {
        let text = "words that. are. followed by. periods. period.";
        let keep = KeepWords::new("are. by.", "", false);
        let markers = ".";

        let broken = insert_linebreaks_between_sentences(text, "|", markers, &keep);

        // We never detect a sentence at and the end of the text.
        let expected = "words that.\n|are. followed by. periods.\n|period.";

        assert_eq!(expected, broken);
    }
}
