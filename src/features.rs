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
    /// Create a FeatureCfg from individual flags
    pub fn from_flags(
        link_actions: Option<crate::cfg::LinkActions>,
        keep_whitespace: Option<crate::cfg::KeepWhitespace>,
        format_block_quotes_flag: bool,
    ) -> Self {
        use crate::cfg::{KeepWhitespace, LinkActions};

        let mut cfg = Self::default();

        // Apply link actions
        if let Some(actions) = link_actions {
            match actions {
                LinkActions::None => {
                    // Do nothing - keep defaults
                }
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

        // Apply whitespace preservation
        if let Some(ws) = keep_whitespace {
            match ws {
                KeepWhitespace::None => {
                    // Do nothing - keep defaults
                }
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

        // Apply block quote formatting
        if format_block_quotes_flag {
            cfg.format_block_quotes = true;
        }

        log::debug!("loaded features: {:?}", cfg);
        cfg
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn link_actions_outsource_inline() {
        use crate::cfg::LinkActions;
        let cfg = FeatureCfg::from_flags(Some(LinkActions::OutsourceInline), None, false);
        assert!(cfg.outsource_inline_links);
        assert!(!cfg.collate_link_defs);
    }

    #[test]
    fn link_actions_collate_defs() {
        use crate::cfg::LinkActions;
        let cfg = FeatureCfg::from_flags(Some(LinkActions::CollateDefs), None, false);
        assert!(!cfg.outsource_inline_links);
        assert!(cfg.collate_link_defs);
    }

    #[test]
    fn link_actions_both() {
        use crate::cfg::LinkActions;
        let cfg = FeatureCfg::from_flags(Some(LinkActions::Both), None, false);
        assert!(cfg.outsource_inline_links);
        assert!(cfg.collate_link_defs);
    }

    #[test]
    fn keep_whitespace_in_links() {
        use crate::cfg::KeepWhitespace;
        let cfg = FeatureCfg::from_flags(None, Some(KeepWhitespace::InLinks), false);
        assert!(cfg.keep_spaces_in_links);
        assert!(!cfg.parse_cfg.keep_linebreaks);
        assert!(!cfg.break_cfg.keep_linebreaks);
    }

    #[test]
    fn keep_whitespace_linebreaks() {
        use crate::cfg::KeepWhitespace;
        let cfg = FeatureCfg::from_flags(None, Some(KeepWhitespace::Linebreaks), false);
        assert!(!cfg.keep_spaces_in_links);
        assert!(cfg.parse_cfg.keep_linebreaks);
        assert!(cfg.break_cfg.keep_linebreaks);
    }

    #[test]
    fn keep_whitespace_both() {
        use crate::cfg::KeepWhitespace;
        let cfg = FeatureCfg::from_flags(None, Some(KeepWhitespace::Both), false);
        assert!(cfg.keep_spaces_in_links);
        assert!(cfg.parse_cfg.keep_linebreaks);
        assert!(cfg.break_cfg.keep_linebreaks);
    }

    #[test]
    fn format_block_quotes_flag() {
        let cfg = FeatureCfg::from_flags(None, None, true);
        assert!(cfg.format_block_quotes);
    }

    #[test]
    fn combining_all_new_flags() {
        use crate::cfg::{KeepWhitespace, LinkActions};
        let cfg = FeatureCfg::from_flags(
            Some(LinkActions::Both),
            Some(KeepWhitespace::Both),
            true,
        );
        assert!(cfg.outsource_inline_links);
        assert!(cfg.collate_link_defs);
        assert!(cfg.keep_spaces_in_links);
        assert!(cfg.parse_cfg.keep_linebreaks);
        assert!(cfg.break_cfg.keep_linebreaks);
        assert!(cfg.format_block_quotes);
    }
}
