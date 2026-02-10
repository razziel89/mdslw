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

use anyhow::{Error, Result};
use include_dir::{Dir, include_dir};

static LANG_FILES_DIR: Dir<'_> = include_dir!("$MDSLW_LANG_DIR");

pub fn keep_word_list(lang_names: &str) -> Result<String> {
    let mut errors = vec![];

    let keep_words = lang_names
        .split_terminator(',')
        .flat_map(|el| el.split_whitespace())
        .filter_map(|el| {
            if el == "none" {
                Some(String::new())
            } else if let Some(content) = LANG_FILES_DIR
                .get_file(el)
                .and_then(|el| el.contents_utf8())
            {
                log::debug!("loaded keep word list for language '{}'", el);
                Some(content.to_string())
            } else {
                errors.push(el);
                None
            }
        })
        .collect::<String>();

    if errors.is_empty() {
        Ok(keep_words)
    } else {
        Err(Error::msg(format!(
            "unknown or unsupported languages: {}",
            errors.join(", ")
        )))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn nothing_disables_words() -> Result<()> {
        let list = keep_word_list("")?;
        assert_eq!(list, String::new());
        Ok(())
    }

    #[test]
    fn none_disables_words() -> Result<()> {
        let list = keep_word_list("none")?;
        assert_eq!(list, String::new());
        Ok(())
    }

    #[test]
    fn some_langs_are_supported() -> Result<()> {
        let langs = "de en es fr it";
        let list = keep_word_list(langs)?;
        assert_ne!(list, String::new());
        Ok(())
    }

    #[test]
    fn unsupported_langs() {
        let langs = "unsupported";
        let list = keep_word_list(langs);
        assert!(list.is_err());
    }
}
