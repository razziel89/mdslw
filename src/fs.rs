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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn listing_non_existent_fails() {
        let is_err = find_files_with_extension(vec!["i do not exist".into()], ".md").is_err();
        assert!(is_err);
    }

    // A struct that will automatically create and delete a temporary directory and that can create
    // arbitrary temporary files underneath it, including their parent dirs.
    struct TempDir(tempfile::TempDir);

    impl TempDir {
        fn new() -> Result<Self> {
            Ok(Self(tempfile::TempDir::new()?))
        }

        fn new_file_in_dir(&self, path: PathBuf) -> Result<PathBuf> {
            let mut result = PathBuf::from(self.0.path());

            // Create directory containing file.
            if let Some(parent) = path.parent() {
                result.extend(parent.into_iter());
                std::fs::create_dir_all(&result)?;
            }

            if let Some(file_name) = path.file_name() {
                result.push(file_name);
                std::fs::File::create(&result)?;
                Ok(result)
            } else {
                Err(Error::msg("no file given"))
            }
        }
    }

    #[test]
    fn finding_all_md_files_in_repo() -> Result<()> {
        let tmp = TempDir::new()?;
        // Create some directory tree that will then be searched.
        tmp.new_file_in_dir("f_1.md".into())?;
        tmp.new_file_in_dir("no_md_1.ext".into())?;
        tmp.new_file_in_dir("no_md_2.ext".into())?;
        tmp.new_file_in_dir("dir/f_2.md".into())?;
        tmp.new_file_in_dir("dir/no_md_1.ext".into())?;
        tmp.new_file_in_dir("other_dir/f_3.md".into())?;
        tmp.new_file_in_dir("other_dir/no_md_1.ext".into())?;

        let ext_found = find_files_with_extension(vec![tmp.0.path().into()], ".ext")?;
        assert_eq!(ext_found.len(), 4);

        let found = find_files_with_extension(vec![tmp.0.path().into()], ".md")?;
        assert_eq!(found.len(), 3);

        Ok(())
    }
}
