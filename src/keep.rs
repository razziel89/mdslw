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
    pub fn new(words: &str, preserve_case: bool) -> Self {
        let cased_words = if preserve_case {
            words.to_owned()
        } else {
            words.to_lowercase()
        };
        Self {
            preserve_case,
            data: cased_words
                .split_whitespace()
                .map(|el| (el.to_string(), el.len()))
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
                // Check whether all characters of the keep word and the slice through the text are
                // identical.
                .any(|(el, disp)| {
                    text[idx - disp..=*idx]
                        .iter()
                        // Convert the text we compare to to lower case, but only those parts that
                        // we actually compare against. The conversion is somewhat annoying and
                        // complicated because a single upper-case character might map to multiple
                        // lower-case ones when converted (not sure why that would be so).
                        .flat_map(|el| {
                            if self.preserve_case {
                                vec![*el]
                            } else {
                                el.to_lowercase().collect::<Vec<_>>()
                            }
                        })
                        // The string self.data is already in lower case if desired. No conversion
                        // needed here. But include the whitespace before the word to avoid
                        // detecting a keep word if only parts of it match. Unfortunately, that
                        // also means that we will still wrap after a keep word if it is the very
                        // first word in the document. TODO: come up with a fix.
                        .zip([' '].into_iter().chain(el.chars()))
                        .all(|(ch1, ch2)| ch1 == ch2)
                })
        } else {
            false
        }
    }
}
