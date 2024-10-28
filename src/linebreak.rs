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

/// Replace all linebreaks by spaces unless they have been escaped by a non-breaking space.
fn normalise_linebreaks(text: &str, detector: &WhitespaceDetector) -> String {
    let mut last_was_nbsp = false;
    text.chars()
        .map(|el| {
            let replacement = if el != '\n' || last_was_nbsp { el } else { ' ' };
            last_was_nbsp = detector.is_nbsp(&el);
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

            if detector.is_breaking_marker(ch, next)
                && !detector.ends_with_keep_word(&as_chars, &idx)
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
    fn normalising_linebreaks() {
        // All whitespace, including tabs, is merged into single spaces.
        let text = " \n text with 	 lots\n \nof   white \n     space    	   ";
        let expected = "   text with 	 lots  \nof   white \n     space    	   ";

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
