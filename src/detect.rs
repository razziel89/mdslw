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

pub struct BreakDetector {
    // Information related to whitespace.
    pub whitespace: WhitespaceDetector,

    // Information related to keep words.
    keep_words: HashSet<(String, usize)>,
    keep_words_preserve_case: bool,

    // Information related to end markers.
    end_markers: String,
    break_multiple_markers: bool,
    break_start_markers: bool,
}

#[derive(Default)]
pub struct WhitespaceDetector {
    nbsp: String,
}

impl WhitespaceDetector {
    pub fn new(keep_non_breaking_spaces: bool) -> Self {
        let nbsp = if keep_non_breaking_spaces {
            // This string contains all three different non-breaking spaces: zero-width, narrow,
            // and normal width.
            String::from("  ﻿")
        } else {
            String::new()
        };

        Self { nbsp }
    }

    pub fn is_whitespace(&self, ch: &char) -> bool {
        // The character is whiespace if it is detected to be UTF8 whitespace and if it is not in
        // the list of excluded whitespace characters known by this struct.
        ch.is_whitespace() && !self.nbsp.contains(*ch)
    }
}

#[derive(Debug, PartialEq)]
pub struct BreakCfg {
    pub breaking_multiple_markers: bool,
    pub breaking_start_marker: bool,
    pub breaking_nbsp: bool,
}

impl BreakDetector {
    pub fn new(
        keep_words: &str,
        keep_word_ignores: &str,
        keep_words_preserve_case: bool,
        end_markers: String,
        break_cfg: &BreakCfg,
    ) -> Self {
        let (cased_words, cased_ignores) = if keep_words_preserve_case {
            (keep_words.to_owned(), keep_word_ignores.to_owned())
        } else {
            (keep_words.to_lowercase(), keep_word_ignores.to_lowercase())
        };

        let ignores = cased_ignores.split_whitespace().collect::<HashSet<_>>();

        Self {
            // Keep words.
            keep_words_preserve_case,
            keep_words: cased_words
                .split_whitespace()
                .filter(|el| !ignores.contains(el))
                .map(|el| (el.to_string(), el.len() - 1))
                .collect::<HashSet<_>>(),
            // End markers.
            end_markers,
            break_multiple_markers: break_cfg.breaking_multiple_markers,
            break_start_markers: break_cfg.breaking_start_marker,
            // Whitspace.
            whitespace: WhitespaceDetector::new(!break_cfg.breaking_nbsp),
        }
    }

    /// Checks whether "text" ends with one of the keep words known by self at "idx".
    pub fn ends_with_keep_word(&self, text: &Vec<char>, idx: &usize) -> bool {
        if idx < &text.len() {
            self.keep_words
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
                            if self.keep_words_preserve_case {
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

    /// Checks whether ch is an end marker and whether the surrounding characters indicate that ch
    /// is actually at the end of a sentence.
    pub fn is_breaking_marker(&self, prev: Option<&char>, ch: &char, next: Option<&char>) -> bool {
        // The current character has to be an end marker. If it is not, it does not end a sentence.
        self.end_markers.contains(*ch)
            // The next character must be whitespace. If it is not, this character is in the middle
            // of a word and, thus, not at the end of a sentence.
            && is_whitespace(next)
            // The previous character must not itself be and end marker. If it is, we only break if
            // we consider multiple successive markers to end sentences.
            && (self.break_multiple_markers || !is_marker(prev, &self.end_markers))
            // The previous character must not be at the beginning of a line. If it is, we oly
            // break if we allow end markers at the beginning of a line.
            && (self.break_start_markers || !is_start(prev))
    }
}

// Some helper functions that make it easier to work with Option<&char> follow.

fn is_marker(ch: Option<&char>, markers: &str) -> bool {
    ch.map(|el| markers.contains(*el)).unwrap_or(false)
}

fn is_start(ch: Option<&char>) -> bool {
    ch.is_none() || ch == Some(&'\n')
}

fn is_whitespace(ch: Option<&char>) -> bool {
    ch.map(|el| el.is_whitespace()).unwrap_or(false)
}

#[cfg(test)]
mod test {
    use super::*;

    const TEXT_FOR_TESTS: &str = "Lorem iPsum doLor SiT aMeT. ConSectEtur adIpiSciNg ELiT.";
    const CFG_FOR_TESTS: &BreakCfg = &BreakCfg {
        breaking_multiple_markers: false,
        breaking_start_marker: false,
        breaking_nbsp: false,
    };

    #[test]
    fn case_insensitive_match() {
        let detector = BreakDetector::new(
            "ipsum sit adipiscing",
            "",
            false,
            "".to_string(),
            CFG_FOR_TESTS,
        );
        let text = TEXT_FOR_TESTS.chars().collect::<Vec<_>>();

        let found = (0..text.len())
            .filter(|el| detector.ends_with_keep_word(&text, el))
            .collect::<Vec<_>>();

        assert_eq!(found, vec![10, 20, 49]);
    }

    #[test]
    fn case_sensitive_match() {
        let detector = BreakDetector::new(
            "ipsum SiT adipiscing",
            "",
            true,
            "".to_string(),
            CFG_FOR_TESTS,
        );
        let text = TEXT_FOR_TESTS.chars().collect::<Vec<_>>();

        let found = (0..text.len())
            .filter(|el| detector.ends_with_keep_word(&text, el))
            .collect::<Vec<_>>();

        assert_eq!(found, vec![20]);
    }

    #[test]
    fn matches_at_start_and_end() {
        let detector = BreakDetector::new("lorem elit.", "", false, "".to_string(), CFG_FOR_TESTS);
        let text = TEXT_FOR_TESTS.chars().collect::<Vec<_>>();

        // Try to search outside the text's range, which will never match.
        let found = (0..text.len() + 5)
            .filter(|el| detector.ends_with_keep_word(&text, el))
            .collect::<Vec<_>>();

        assert_eq!(found, vec![4, 55]);
    }

    #[test]
    fn ignoring_words_case_sensitively() {
        let detector = BreakDetector::new(
            "ipsum SiT adipiscing",
            "SiT",
            true,
            "".to_string(),
            CFG_FOR_TESTS,
        );
        let text = TEXT_FOR_TESTS.chars().collect::<Vec<_>>();

        let found = (0..text.len())
            .filter(|el| detector.ends_with_keep_word(&text, el))
            .collect::<Vec<_>>();

        assert_eq!(found, vec![]);
    }

    #[test]
    fn ignoring_words_case_insensitively() {
        let detector = BreakDetector::new(
            "ipsum sit adipiscing",
            "sit",
            false,
            "".to_string(),
            CFG_FOR_TESTS,
        );
        let text = TEXT_FOR_TESTS.chars().collect::<Vec<_>>();

        let found = (0..text.len())
            .filter(|el| detector.ends_with_keep_word(&text, el))
            .collect::<Vec<_>>();

        assert_eq!(found, vec![10, 49]);
    }

    #[test]
    fn ingores_that_are_no_suppressions_are_ignored() {
        let detector = BreakDetector::new(
            "ipsum sit adipiscing",
            "sit asdf blub muhaha",
            false,
            "".to_string(),
            CFG_FOR_TESTS,
        );
        let text = TEXT_FOR_TESTS.chars().collect::<Vec<_>>();

        let found = (0..text.len())
            .filter(|el| detector.ends_with_keep_word(&text, el))
            .collect::<Vec<_>>();

        assert_eq!(found, vec![10, 49]);
    }
}
