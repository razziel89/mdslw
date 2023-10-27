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

pub struct KeepWords {
    data: HashSet<(String, usize)>,
    preserve_case: bool,
}

impl KeepWords {
    pub fn new(words: &str, ignores: &str, preserve_case: bool) -> Self {
        let (cased_words, cased_ignores) = if preserve_case {
            (words.to_owned(), ignores.to_owned())
        } else {
            (words.to_lowercase(), ignores.to_lowercase())
        };

        let ignores = cased_ignores.split_whitespace().collect::<HashSet<_>>();

        Self {
            preserve_case,
            data: cased_words
                .split_whitespace()
                .filter(|el| !ignores.contains(el))
                .map(|el| (el.to_string(), el.len() - 1))
                .collect::<HashSet<_>>(),
        }
    }

    /// Checks whether "text" ends with one of the keep words known by self at "idx".
    pub fn ends_with_word(&self, text: &Vec<char>, idx: &usize) -> bool {
        if idx < &text.len() {
            self.data
                .iter()
                // Only check words that can actually be in the text.
                .filter(|(_el, disp)| idx >= disp)
                // Determine whether any keep word matches.
                .any(|(el, disp)| {
                    // Check whether the word is at the start of the text or whether it is preceded
                    // by a character that is not alphanumeric. That way, we avoid matching a keep
                    // word of "g." on a text going "e.g.". Note that, here, idx>=disp holds.
                    (idx == disp || !text[idx - disp -1].is_alphanumeric()) &&
                    // Check whether all characters of the keep word and the slice through the text
                    // are identical.
                    text[idx - disp..=*idx]
                        .iter()
                        // Convert the text we compare with to lower case, but only those parts
                        // that we actually do compare with. The conversion is somewhat annoying
                        // and complicated because a single upper-case character might map to
                        // multiple lower-case ones when converted (not sure why that would be so).
                        .flat_map(|el| {
                            if self.preserve_case {
                                vec![*el]
                            } else {
                                el.to_lowercase().collect::<Vec<_>>()
                            }
                        })
                        // The strings self.data is already in lower case if desired. No conversion
                        // needed here.
                        .zip(el.chars())
                        .all(|(ch1, ch2)| ch1 == ch2)
                })
        } else {
            false
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const TEXT_FOR_TESTS: &str = "Lorem iPsum doLor SiT aMeT. ConSectEtur adIpiSciNg ELiT.";

    #[test]
    fn case_insensitive_match() {
        let keep = KeepWords::new("ipsum sit adipiscing", "", false);
        let text = TEXT_FOR_TESTS.chars().collect::<Vec<_>>();

        let found = (0..text.len())
            .filter(|el| keep.ends_with_word(&text, el))
            .collect::<Vec<_>>();

        assert_eq!(found, vec![10, 20, 49]);
    }

    #[test]
    fn case_sensitive_match() {
        let keep = KeepWords::new("ipsum SiT adipiscing", "", true);
        let text = TEXT_FOR_TESTS.chars().collect::<Vec<_>>();

        let found = (0..text.len())
            .filter(|el| keep.ends_with_word(&text, el))
            .collect::<Vec<_>>();

        assert_eq!(found, vec![20]);
    }

    #[test]
    fn matches_at_start_and_end() {
        let keep = KeepWords::new("lorem elit.", "", false);
        let text = TEXT_FOR_TESTS.chars().collect::<Vec<_>>();

        // Try to search outside the text's range, which will never match.
        let found = (0..text.len() + 5)
            .filter(|el| keep.ends_with_word(&text, el))
            .collect::<Vec<_>>();

        assert_eq!(found, vec![4, 55]);
    }

    #[test]
    fn ignoring_words_case_sensitively() {
        let keep = KeepWords::new("ipsum SiT adipiscing", "SiT", true);
        let text = TEXT_FOR_TESTS.chars().collect::<Vec<_>>();

        let found = (0..text.len())
            .filter(|el| keep.ends_with_word(&text, el))
            .collect::<Vec<_>>();

        assert_eq!(found, vec![]);
    }

    #[test]
    fn ignoring_words_case_insensitively() {
        let keep = KeepWords::new("ipsum sit adipiscing", "sit", false);
        let text = TEXT_FOR_TESTS.chars().collect::<Vec<_>>();

        let found = (0..text.len())
            .filter(|el| keep.ends_with_word(&text, el))
            .collect::<Vec<_>>();

        assert_eq!(found, vec![10, 49]);
    }

    #[test]
    fn ingores_that_are_no_suppressions_are_ignored() {
        let keep = KeepWords::new("ipsum sit adipiscing", "sit asdf blub muhaha", false);
        let text = TEXT_FOR_TESTS.chars().collect::<Vec<_>>();

        let found = (0..text.len())
            .filter(|el| keep.ends_with_word(&text, el))
            .collect::<Vec<_>>();

        assert_eq!(found, vec![10, 49]);
    }
}
