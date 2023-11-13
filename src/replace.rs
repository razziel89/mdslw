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

use pulldown_cmark::{Event, Parser, Tag};
use std::collections::HashSet;

pub fn replace_spaces_in_links_by_nbsp(text: String) -> String {
    let char_indices_in_links = Parser::new(&text)
        .into_offset_iter()
        .filter_map(|(event, range)| match event {
            Event::Start(Tag::Link(..)) => Some(range),
            _ => None,
        })
        .flatten()
        .collect::<HashSet<_>>();

    let mut last_replaced = false;
    text.char_indices()
        .filter_map(|(idx, ch)| {
            if ch.is_whitespace() && char_indices_in_links.contains(&idx) {
                if last_replaced {
                    None
                } else {
                    last_replaced = true;
                    Some('\u{00a0}')
                }
            } else {
                last_replaced = false;
                Some(ch)
            }
        })
        .collect::<String>()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn replacing_spaces_only_in_links() {
        let original = "Outside of link, [inside of link](http://some-url), again outside.";
        let expected = "Outside of link, [inside of link](http://some-url), again outside.";

        let replaced = replace_spaces_in_links_by_nbsp(original.to_string());

        assert_eq!(replaced, expected);
    }
}
