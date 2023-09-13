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

pub fn build_indent(num: usize) -> String {
    (0..num).map(|_| ' ').collect::<String>()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn can_build_indents() {
        let three = build_indent(3);
        assert_eq!(three, String::from("   "));

        let four = build_indent(4);
        assert_eq!(four, String::from("    "));
    }
}
