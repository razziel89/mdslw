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

use crate::detect::{BreakDetector, WhitespaceDetector};

pub fn insert_linebreaks_after_sentence_ends(text: &str, detector: &BreakDetector) -> String {
    let merged = normalise_linebreaks(text, &detector.whitespace);
    let sentence_ends = find_sentence_ends(&merged, detector);

    merged
        .chars()
        .enumerate()
        .filter_map(|(idx, el)| {
            if sentence_ends.contains(&Char::Skip(idx)) {
                None
            } else if sentence_ends.contains(&Char::Split(idx)) {
                Some(format!("\n{}", el))
            } else {
                Some(format!("{}", el))
            }
        })
        .collect::<String>()
}

/// Replace all linebreaks by spaces unless they have been escaped by a non-breaking space, a
/// backslash, or at least two preceding spaces.
fn normalise_linebreaks(text: &str, detector: &WhitespaceDetector) -> String {
    let mut last_was_nbsp = false;
    let mut last_was_backslash = false;
    let mut number_of_preceding_spaces: usize = 0;
    text.chars()
        .map(|el| {
            let replacement = if el != '\n'
                || last_was_nbsp
                || last_was_backslash
                || number_of_preceding_spaces >= 2
            {
                el
            } else {
                ' '
            };
            last_was_nbsp = detector.is_nbsp(&el);
            last_was_backslash = el == '\\';
            if el == ' ' {
                number_of_preceding_spaces += 1;
            } else {
                number_of_preceding_spaces = 0;
            }
            replacement
        })
        .collect::<String>()
}

#[derive(Eq, Hash, PartialEq, Debug)]
enum Char {
    Skip(usize),
    Split(usize),
}

fn find_sentence_ends(text: &str, detector: &BreakDetector) -> HashSet<Char> {
    let as_chars = text.chars().collect::<Vec<_>>();

    as_chars
        .iter()
        .enumerate()
        .filter_map(|(idx, ch)| {
            let next = as_chars.get(idx + 1);
            let mut count: usize = 0;

            if detector.is_breaking_marker(ch, next)
                && !detector.ends_with_keep_word(&as_chars, &idx)
                && !(
                    // Check whether this end of a sentence is followed by a hard line break
                    // represented by at least two spaces followed by a linebreak. We find the next
                    // character that is no space. If that is a linebreak that was preceeded by at
                    // least two spaces, we don't add a line break.
                    as_chars[idx..].iter().skip(1).find(|ch| {
                        if ch == &&' ' {
                            count += 1;
                            false
                        } else {
                            true
                        }
                    }) == Some(&'\n')
                        && count >= 2
                )
            {
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
    use crate::detect::BreakCfg;

    const CFG_FOR_TESTS: &BreakCfg = &BreakCfg {
        keep_linebreaks: false,
    };

    #[test]
    fn finding_sentence_ends() {
        let text = "words that. are. followed by. periods. period.";
        let detector = BreakDetector::new("are. by.", "", false, ".", CFG_FOR_TESTS);

        let ends = find_sentence_ends(text, &detector);

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
    fn finding_sentence_ends_with_hard_breaks() {
        let text = "words that.  \nare. followed by.  \nperiods. period.";
        let detector = BreakDetector::new("", "", false, ".", CFG_FOR_TESTS);

        let ends = find_sentence_ends(text, &detector);

        // We never detect a sentence at and the end of the text.
        let expected = vec![
            Char::Skip(18),
            Char::Split(19),
            Char::Skip(42),
            Char::Split(43),
        ]
        .into_iter()
        .collect::<HashSet<_>>();

        assert_eq!(expected, ends);
    }

    #[test]
    fn normalising_linebreaks() {
        // All whitespace, including tabs, is merged into single spaces.
        let text = " \n text with 	 lots\n \nof   white \n     space    	   ";
        let expected = "   text with 	 lots  \nof   white \n     space    	   ";

        let merged = normalise_linebreaks(text, &WhitespaceDetector::default());

        assert_eq!(expected, merged);
    }

    #[test]
    fn normalising_linebreaks_keeping_hard_breaks() {
        // A backslash or at least two spaces at the end of a line are preserved.
        let text = " \n text with 	 lots\\\n \nof   white    \n     space    	   ";
        let expected = "   text with 	 lots\\\n \nof   white    \n     space    	   ";

        let merged = normalise_linebreaks(text, &WhitespaceDetector::default());

        assert_eq!(expected, merged);
    }

    #[test]
    fn inserting_linebreaks_between_sentences() {
        let text = "words that. are. followed by. periods. period.";
        let detector = BreakDetector::new("are. by.", "", false, ".", CFG_FOR_TESTS);

        let broken = insert_linebreaks_after_sentence_ends(text, &detector);

        // We never detect a sentence at and the end of the text.
        let expected = "words that.\nare. followed by. periods.\nperiod.";

        assert_eq!(expected, broken);
    }
}
