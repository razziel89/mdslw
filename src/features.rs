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

use crate::detect::BreakCfg;
use crate::parse::ParseCfg;

#[derive(Debug, PartialEq)]
pub struct FeatureCfg {
    pub keep_spaces_in_links: bool,
    pub break_cfg: BreakCfg,
    pub parse_cfg: ParseCfg,
}

impl Default for FeatureCfg {
    fn default() -> Self {
        FeatureCfg {
            keep_spaces_in_links: false,
            parse_cfg: ParseCfg {
                keep_inline_html: false,
                keep_footnotes: false,
                keep_tasklists: true,
                keep_tables: true,
                keep_nbsp: true,
                keep_newlines: false,
            },
            break_cfg: BreakCfg {
                breaking_multiple_markers: false,
                breaking_start_marker: false,
                breaking_nbsp: false,
                keep_newlines: false,
            },
        }
    }
}

impl std::str::FromStr for FeatureCfg {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut cfg = Self::default();
        let mut errors = vec![];

        // Parse all possible features and toggle them as desired.
        for feature in s
            .split_terminator(',')
            .flat_map(|el| el.split_whitespace())
            .map(|el| el.trim())
            .filter(|el| !el.is_empty())
        {
            match feature {
                "keep-spaces-in-links" => cfg.keep_spaces_in_links = true,
                "keep-inline-html" => cfg.parse_cfg.keep_inline_html = true,
                "keep-footnotes" => cfg.parse_cfg.keep_footnotes = true,
                "modify-nbsp" => {
                    cfg.parse_cfg.keep_nbsp = false;
                    cfg.break_cfg.breaking_nbsp = true
                }
                "modify-tasklists" => cfg.parse_cfg.keep_tasklists = false,
                "modify-tables" => cfg.parse_cfg.keep_tables = false,
                "breaking-multiple-markers" => cfg.break_cfg.breaking_multiple_markers = true,
                "breaking-start-marker" => cfg.break_cfg.breaking_start_marker = true,
                "keep-newlines" => {
                    cfg.parse_cfg.keep_newlines = true;
                    cfg.break_cfg.keep_newlines = true;
                }
                // Do not accept any other entry.
                _ => errors.push(feature),
            }
        }

        if errors.is_empty() {
            log::debug!("loaded features: {:?}", cfg);
            Ok(cfg)
        } else {
            Err(Error::msg(format!(
                "unknown features: {}",
                errors.join(", ")
            )))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn swapping_all_features_and_disregard_whitspace() -> Result<()> {
        let default = FeatureCfg::default();
        let swapped = FeatureCfg {
            keep_spaces_in_links: !default.keep_spaces_in_links,
            parse_cfg: ParseCfg {
                keep_inline_html: !default.parse_cfg.keep_inline_html,
                keep_footnotes: !default.parse_cfg.keep_footnotes,
                keep_tasklists: !default.parse_cfg.keep_tasklists,
                keep_tables: !default.parse_cfg.keep_tables,
                keep_nbsp: !default.parse_cfg.keep_nbsp,
                keep_newlines: !default.parse_cfg.keep_newlines,
            },
            break_cfg: BreakCfg {
                breaking_multiple_markers: !default.break_cfg.breaking_multiple_markers,
                breaking_start_marker: !default.break_cfg.breaking_start_marker,
                breaking_nbsp: !default.break_cfg.breaking_nbsp,
                keep_newlines: !default.break_cfg.keep_newlines,
            },
        };

        let parsed = "keep-inline-html, keep-footnotes , modify-tasklists, modify-tables, \
            breaking-multiple-markers, breaking-start-marker, modify-nbsp, keep-spaces-in-links \
            retain-whitespace"
            .parse::<FeatureCfg>()?;

        assert_eq!(parsed, swapped);
        Ok(())
    }

    #[test]
    fn failure_to_parse() -> Result<()> {
        let parsed = "unknown".parse::<FeatureCfg>();
        assert!(parsed.is_err());
        Ok(())
    }
}
