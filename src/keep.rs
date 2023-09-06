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

use std::collections::{HashMap, HashSet};

pub struct KeepWords {
    data: HashSet<(String, usize)>,
}

impl KeepWords {
    pub fn new(words: &str) -> Self {
        Self {
            data: words
                .split_whitespace()
                .map(|el| (el.to_string(), el.len() - 1))
                .collect::<HashSet<_>>(),
        }
    }

    /// Checks whether "text" ends with one of the keep words known by self at "idx".
    pub fn ends_with_word(&self, text: &Vec<char>, idx: &usize) -> bool {
        if idx + 1 < text.len() {
            self.data
                .iter()
                // Only check words that can actually be in the text.
                .filter(|(_el, disp)| idx >= disp)
                // Check whether all characters of the keep word and the slice through the text are
                // identical.
                .any(|(el, disp)| {
                    text[idx - disp..=*idx]
                        .iter()
                        .zip(el.chars())
                        .all(|(ch1, ch2)| ch1 == &ch2)
                })
        } else {
            false
        }
    }
}
