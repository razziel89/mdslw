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
    pub format_block_quotes: bool,
    pub collate_link_defs: bool,
    pub outsource_inline_links: bool,
    pub break_cfg: BreakCfg,
    pub parse_cfg: ParseCfg,
}

impl Default for FeatureCfg {
    fn default() -> Self {
        FeatureCfg {
            keep_spaces_in_links: false,
            format_block_quotes: false,
            collate_link_defs: false,
            outsource_inline_links: false,
            parse_cfg: ParseCfg {
                keep_linebreaks: false,
            },
            break_cfg: BreakCfg {
                keep_linebreaks: false,
            },
        }
    }
}

impl FeatureCfg {
    /// Create a FeatureCfg from individual flags and legacy features string
    pub fn from_flags(
        features_str: &str,
        link_actions: Option<crate::cfg::LinkActions>,
        keep_whitespace: Option<crate::cfg::KeepWhitespace>,
        format_block_quotes_flag: bool,
    ) -> Result<Self> {
        use crate::cfg::{KeepWhitespace, LinkActions};

        let mut cfg = Self::default();
        let mut errors = vec![];

        // First, parse legacy features string for backward compatibility
        for feature in features_str
            .split_terminator(',')
            .flat_map(|el| el.split_whitespace())
            .map(|el| el.trim())
            .filter(|el| !el.is_empty())
        {
            match feature {
                "keep-spaces-in-links" => cfg.keep_spaces_in_links = true,
                "format-block-quotes" => cfg.format_block_quotes = true,
                "collate-link-defs" => cfg.collate_link_defs = true,
                "outsource-inline-links" => cfg.outsource_inline_links = true,
                "keep-linebreaks" => {
                    cfg.parse_cfg.keep_linebreaks = true;
                    cfg.break_cfg.keep_linebreaks = true;
                }
                // Do not accept any other entry.
                _ => errors.push(feature),
            }
        }

        // Apply new flags (these override the legacy features)
        if let Some(actions) = link_actions {
            match actions {
                LinkActions::OutsourceInline => {
                    cfg.outsource_inline_links = true;
                }
                LinkActions::CollateDefs => {
                    cfg.collate_link_defs = true;
                }
                LinkActions::Both => {
                    cfg.outsource_inline_links = true;
                    cfg.collate_link_defs = true;
                }
            }
        }

        if let Some(ws) = keep_whitespace {
            match ws {
                KeepWhitespace::InLinks => {
                    cfg.keep_spaces_in_links = true;
                }
                KeepWhitespace::Linebreaks => {
                    cfg.parse_cfg.keep_linebreaks = true;
                    cfg.break_cfg.keep_linebreaks = true;
                }
                KeepWhitespace::Both => {
                    cfg.keep_spaces_in_links = true;
                    cfg.parse_cfg.keep_linebreaks = true;
                    cfg.break_cfg.keep_linebreaks = true;
                }
            }
        }

        if format_block_quotes_flag {
            cfg.format_block_quotes = true;
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

impl std::str::FromStr for FeatureCfg {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::from_flags(s, None, None, false)
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
            format_block_quotes: !default.format_block_quotes,
            collate_link_defs: !default.collate_link_defs,
            outsource_inline_links: !default.outsource_inline_links,
            parse_cfg: ParseCfg {
                keep_linebreaks: !default.parse_cfg.keep_linebreaks,
            },
            break_cfg: BreakCfg {
                keep_linebreaks: !default.break_cfg.keep_linebreaks,
            },
        };

        let parsed =
            "keep-spaces-in-links , keep-linebreaks ,format-block-quotes, collate-link-defs,outsource-inline-links"
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

    #[test]
    fn link_actions_outsource_inline() -> Result<()> {
        use crate::cfg::LinkActions;
        let cfg = FeatureCfg::from_flags("", Some(LinkActions::OutsourceInline), None, false)?;
        assert!(cfg.outsource_inline_links);
        assert!(!cfg.collate_link_defs);
        Ok(())
    }

    #[test]
    fn link_actions_collate_defs() -> Result<()> {
        use crate::cfg::LinkActions;
        let cfg = FeatureCfg::from_flags("", Some(LinkActions::CollateDefs), None, false)?;
        assert!(!cfg.outsource_inline_links);
        assert!(cfg.collate_link_defs);
        Ok(())
    }

    #[test]
    fn link_actions_both() -> Result<()> {
        use crate::cfg::LinkActions;
        let cfg = FeatureCfg::from_flags("", Some(LinkActions::Both), None, false)?;
        assert!(cfg.outsource_inline_links);
        assert!(cfg.collate_link_defs);
        Ok(())
    }

    #[test]
    fn keep_whitespace_in_links() -> Result<()> {
        use crate::cfg::KeepWhitespace;
        let cfg = FeatureCfg::from_flags("", None, Some(KeepWhitespace::InLinks), false)?;
        assert!(cfg.keep_spaces_in_links);
        assert!(!cfg.parse_cfg.keep_linebreaks);
        assert!(!cfg.break_cfg.keep_linebreaks);
        Ok(())
    }

    #[test]
    fn keep_whitespace_linebreaks() -> Result<()> {
        use crate::cfg::KeepWhitespace;
        let cfg = FeatureCfg::from_flags("", None, Some(KeepWhitespace::Linebreaks), false)?;
        assert!(!cfg.keep_spaces_in_links);
        assert!(cfg.parse_cfg.keep_linebreaks);
        assert!(cfg.break_cfg.keep_linebreaks);
        Ok(())
    }

    #[test]
    fn keep_whitespace_both() -> Result<()> {
        use crate::cfg::KeepWhitespace;
        let cfg = FeatureCfg::from_flags("", None, Some(KeepWhitespace::Both), false)?;
        assert!(cfg.keep_spaces_in_links);
        assert!(cfg.parse_cfg.keep_linebreaks);
        assert!(cfg.break_cfg.keep_linebreaks);
        Ok(())
    }

    #[test]
    fn format_block_quotes_flag() -> Result<()> {
        let cfg = FeatureCfg::from_flags("", None, None, true)?;
        assert!(cfg.format_block_quotes);
        Ok(())
    }

    #[test]
    fn new_flags_override_legacy_features() -> Result<()> {
        use crate::cfg::LinkActions;
        // Legacy string enables outsource-inline-links, but new flag overrides to collate-defs
        let cfg = FeatureCfg::from_flags(
            "outsource-inline-links",
            Some(LinkActions::CollateDefs),
            None,
            false,
        )?;
        // New flag wins
        assert!(cfg.collate_link_defs);
        // But legacy is still applied first
        assert!(cfg.outsource_inline_links);
        Ok(())
    }

    #[test]
    fn combining_all_new_flags() -> Result<()> {
        use crate::cfg::{KeepWhitespace, LinkActions};
        let cfg = FeatureCfg::from_flags(
            "",
            Some(LinkActions::Both),
            Some(KeepWhitespace::Both),
            true,
        )?;
        assert!(cfg.outsource_inline_links);
        assert!(cfg.collate_link_defs);
        assert!(cfg.keep_spaces_in_links);
        assert!(cfg.parse_cfg.keep_linebreaks);
        assert!(cfg.break_cfg.keep_linebreaks);
        assert!(cfg.format_block_quotes);
        Ok(())
    }
}
