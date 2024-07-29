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
                keep_linebreaks: false,
            },
            break_cfg: BreakCfg {
                keep_linebreaks: false,
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
                "keep-linebreaks" => {
                    cfg.parse_cfg.keep_linebreaks = true;
                    cfg.break_cfg.keep_linebreaks = true;
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
                keep_linebreaks: !default.parse_cfg.keep_linebreaks,
            },
            break_cfg: BreakCfg {
                keep_linebreaks: !default.break_cfg.keep_linebreaks,
            },
        };

        let parsed = "keep-spaces-in-links , keep-linebreaks".parse::<FeatureCfg>()?;

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
