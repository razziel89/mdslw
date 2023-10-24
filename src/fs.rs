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
use ignore::Walk;

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
                    Walk::new(&el)
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
    use std::collections::HashSet;

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

        fn new_file_in_dir_with_content(&self, path: PathBuf, content: &str) -> Result<PathBuf> {
            let path = self.new_file_in_dir(path)?;
            std::fs::write(&path, content.as_bytes())?;
            Ok(path)
        }

        /// Remove the temporary directory from the prefix.
        fn strip(&self, path: PathBuf) -> PathBuf {
            path.as_path()
                .strip_prefix(self.0.path())
                .unwrap_or(&path)
                .to_path_buf()
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

    #[test]
    fn auto_ignoring_files() -> Result<()> {
        let tmp = TempDir::new()?;
        // Create some directory tree that will then be searched.
        tmp.new_file_in_dir("f.md".into())?;
        tmp.new_file_in_dir("file.md".into())?;
        tmp.new_file_in_dir("stuff.md".into())?;
        tmp.new_file_in_dir("dir/f.md".into())?;
        tmp.new_file_in_dir("dir/file.md".into())?;
        tmp.new_file_in_dir("dir/stuff.md".into())?;
        tmp.new_file_in_dir("dir/fstuff.md".into())?;
        tmp.new_file_in_dir("other_dir/f.md".into())?;
        tmp.new_file_in_dir("other_dir/file.md".into())?;
        tmp.new_file_in_dir("other_dir/stuff.md".into())?;
        tmp.new_file_in_dir("other_dir/fstuff.md".into())?;

        tmp.new_file_in_dir_with_content(".ignore".into(), &format!("stuff.md\n"))?;
        tmp.new_file_in_dir_with_content("dir/.ignore".into(), &format!("file.md\n"))?;
        tmp.new_file_in_dir_with_content("other_dir/.ignore".into(), &format!("f*.md\n"))?;

        let found = find_files_with_extension(vec![tmp.0.path().into()], ".md")?
            .into_iter()
            .map(|el| tmp.strip(el))
            .map(|el| el.to_string_lossy().to_string())
            .collect::<HashSet<_>>();

        let expected = vec!["file.md", "f.md", "dir/fstuff.md", "dir/f.md"]
            .into_iter()
            .map(|el| el.to_string())
            .collect::<HashSet<_>>();

        assert_eq!(found, expected);

        Ok(())
    }
}
