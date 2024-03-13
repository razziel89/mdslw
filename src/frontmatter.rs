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
        let mut matter_len = FRONTMATTER_SEPARATOR.len();
        let mut found_end_sep = false;
        lines
            .take_while(|line| {
                let do_continue = !found_end_sep;
                found_end_sep |= line == &FRONTMATTER_SEPARATOR;
                do_continue
            })
            .for_each(|line| matter_len += line.len());
        if !found_end_sep {
            // There was no frontmatter since we did not find the end separator.
            (String::new(), text)
        } else {
            // There was indeed frontmatter. This slicing operation can never error out sinc we did
            // extract the frontmatter from the text.
            let matter = &text[..matter_len];
            let rest = &text[matter_len..text.len()];
            (matter.to_owned(), rest.to_owned())
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
