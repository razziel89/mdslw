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

use std::path::PathBuf;

use anyhow::{Error, Result};
use walkdir::WalkDir;

pub fn find_files_with_extension(paths: Vec<PathBuf>, extension: &str) -> Result<Vec<PathBuf>> {
    let mut errors = vec![];

    let found = paths
        .into_iter()
        .filter_map(|el| {
            if el.is_file() {
                Some(vec![el])
            } else if el.is_dir() {
                Some(
                    // Recursively extract all files with the given extension.
                    WalkDir::new(&el)
                        .into_iter()
                        .filter_map(|el| match el {
                            Ok(path) => Some(path),
                            Err(err) => {
                                eprintln!("{}", err);
                                None
                            }
                        })
                        .filter_map(|el| match el.path().canonicalize() {
                            Ok(path) => Some(path),
                            Err(err) => {
                                eprintln!("{}: {}", err, el.path().to_string_lossy());
                                None
                            }
                        })
                        // Only keep actual markdown files and symlinks to them.
                        .filter(|el| el.is_file() && el.to_string_lossy().ends_with(extension))
                        .collect::<Vec<_>>(),
                )
            } else {
                errors.push(format!("failed to find path: {}", el.to_string_lossy()));
                None
            }
        })
        .flatten()
        .collect::<Vec<_>>();

    if errors.len() == 0 {
        Ok(found)
    } else {
        Err(Error::msg(format!("{}", errors.join("\n"),)))
    }
}
