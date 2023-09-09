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

pub fn keep_word_list(lang_names: &str) -> Result<String> {
    let mut errors = vec![];

    let keep_words = lang_names
        .split_whitespace()
        .filter_map(|el| match el {
            "de" => Some(String::from_utf8_lossy(include_bytes!("lang/de"))),
            "en" => Some(String::from_utf8_lossy(include_bytes!("lang/en"))),
            "es" => Some(String::from_utf8_lossy(include_bytes!("lang/es"))),
            "fr" => Some(String::from_utf8_lossy(include_bytes!("lang/fr"))),
            "it" => Some(String::from_utf8_lossy(include_bytes!("lang/it"))),
            "none" => Some(String::new().into()),
            _ => {
                errors.push(format!("unknown or unsupported language {}", el));
                None
            }
        })
        .collect::<String>();

    if errors.len() == 0 {
        Ok(keep_words)
    } else {
        Err(Error::msg(errors.join("\n")))
    }
}
