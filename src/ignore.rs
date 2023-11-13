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

const IGNORE_START: &str = "mdslw-ignore-start";
const IGNORE_END: &str = "mdslw-ignore-end";

const PRETTIER_IGNORE_START: &str = "prettier-ignore-start";
const PRETTIER_IGNORE_END: &str = "prettier-ignore-end";

fn is_html_comment(s: &str) -> bool {
    s.starts_with("<!--") && (s.ends_with("-->") || s.ends_with("-->\n"))
}

pub struct IgnoreByHtmlComment {
    ignore: bool,
}

impl IgnoreByHtmlComment {
    pub fn new() -> Self {
        Self { ignore: false }
    }

    /// Determine whether the HTML that is processed is a comment and whether it modifies the
    /// ignore behaviour.
    pub fn process_html(&mut self, s: &str) {
        if is_html_comment(s) {
            if s.contains(IGNORE_START) || s.contains(PRETTIER_IGNORE_START) {
                self.ignore = true
            }
            if s.contains(IGNORE_END) || s.contains(PRETTIER_IGNORE_END) {
                self.ignore = false
            }
        }
    }

    pub fn should_be_ignored(&self) -> bool {
        self.ignore
    }
}
