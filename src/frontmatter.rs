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

const FRONTMATTER_SEPARATOR: &str = "---\n";

pub fn split_frontmatter(text: String) -> (String, String) {
    let mut lines = text.split_inclusive('\n');
    let first = lines.next();
    if Some(FRONTMATTER_SEPARATOR) != first {
        (String::new(), text)
    } else {
        // Re-create first line.
        let mut matter = String::from(FRONTMATTER_SEPARATOR);
        // Add all other lines in the frontmatter.
        let mut found_end_sep = false;
        lines
            .map_while(|line| {
                if line != FRONTMATTER_SEPARATOR {
                    Some(line)
                } else {
                    found_end_sep = true;
                    None
                }
            })
            .for_each(|line| matter.push_str(line));
        if !found_end_sep {
            // There was no frontmatter since we did not find the end separator.
            (String::new(), text)
        } else {
            matter.push_str(FRONTMATTER_SEPARATOR);
            // There was indeed frontmatter. This slicing operation can never error out sinc we did
            // extract the frontmatter from the text.
            let rest = &text[matter.len()..text.len()];
            (matter, rest.to_owned())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const FRONTMATTER_FOR_TEST: &str = "---\nsome text\nasdf: ---\nmultiple: lines\n---\n";

    #[test]
    fn splitting_frontmatter() {
        let (matter, rest) = split_frontmatter(FRONTMATTER_FOR_TEST.to_string());

        assert_eq!(matter, FRONTMATTER_FOR_TEST.to_string());
        assert_eq!(rest, String::new())
    }

    #[test]
    fn splitting_frontmatter_with_rest() {
        let (matter, rest) =
            split_frontmatter(format!("{}some\nmore\ntext\n", FRONTMATTER_FOR_TEST));

        assert_eq!(matter, FRONTMATTER_FOR_TEST.to_string());
        assert_eq!(rest, "some\nmore\ntext\n")
    }

    #[test]
    fn frontmatter_has_to_start_text() {
        let text = format!("something\n{}", FRONTMATTER_FOR_TEST);
        let (matter, rest) = split_frontmatter(text.clone());

        assert_eq!(matter, String::new());
        assert_eq!(rest, text);
    }

    #[test]
    fn frontmatter_has_to_have_ending_separator() {
        let text = FRONTMATTER_FOR_TEST[..FRONTMATTER_FOR_TEST.len() - 1].to_string();
        let (matter, rest) = split_frontmatter(text.clone());

        assert_eq!(matter, String::new());
        assert_eq!(rest, text);
    }
}
