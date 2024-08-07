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

use std::path::Path;

use similar::{udiff::unified_diff, Algorithm};

const CONTEXT: usize = 4;

pub enum DiffAlgo {
    Myers,
    Patience,
    Lcs,
}

impl DiffAlgo {
    fn to_internal(&self) -> Algorithm {
        match self {
            Self::Myers => Algorithm::Myers,
            Self::Patience => Algorithm::Patience,
            Self::Lcs => Algorithm::Lcs,
        }
    }

    pub fn generate(&self, new: &str, org: &str, filename: &Path) -> String {
        let original = format!("original:{}", filename.to_string_lossy());
        let processed = format!("processed:{}", filename.to_string_lossy());
        let names = (original.as_ref(), processed.as_ref());
        unified_diff(self.to_internal(), org, new, CONTEXT, Some(names))
    }
}
