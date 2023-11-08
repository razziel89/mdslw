use anyhow::{Error, Result};

use crate::keep::BreakCfg;
use crate::parse::ParseCfg;

#[derive(Debug, PartialEq)]
pub struct FeatureCfg {
    pub break_cfg: BreakCfg,
    pub parse_cfg: ParseCfg,
}

impl Default for FeatureCfg {
    fn default() -> Self {
        FeatureCfg {
            parse_cfg: ParseCfg {
                keep_inline_html: false,
                keep_footnotes: false,
                keep_tasklists: true,
                keep_tables: true,
            },
            break_cfg: BreakCfg {
                breaking_multiple_markers: false,
                breaking_start_marker: false,
            },
        }
    }
}

impl std::str::FromStr for FeatureCfg {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut cfg = Self::default();
        // Parse all possible features and toggle them as desired.
        for feature in s
            .split_terminator(',')
            .flat_map(|el| el.split_whitespace())
            .map(|el| el.trim())
            .filter(|el| !el.is_empty())
        {
            match feature {
                "keep-inline-html" => cfg.parse_cfg.keep_inline_html = true,
                "keep-footnotes" => cfg.parse_cfg.keep_footnotes = true,
                "modify-tasklists" => cfg.parse_cfg.keep_tasklists = false,
                "modify-tables" => cfg.parse_cfg.keep_tables = false,
                "breaking-multiple-markers" => cfg.break_cfg.breaking_multiple_markers = true,
                "breaking-start-marker" => cfg.break_cfg.breaking_start_marker = true,
                // Do not accept any other entry.
                _ => return Err(Error::msg(format!("unknown parse option '{}'", feature))),
            }
        }
        Ok(cfg)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn swapping_all_features_and_disregard_whitspace() -> Result<()> {
        let default = FeatureCfg::default();
        let swapped = FeatureCfg {
            parse_cfg: ParseCfg {
                keep_inline_html: !default.parse_cfg.keep_inline_html,
                keep_footnotes: !default.parse_cfg.keep_footnotes,
                keep_tasklists: !default.parse_cfg.keep_tasklists,
                keep_tables: !default.parse_cfg.keep_tables,
            },
            break_cfg: BreakCfg {
                breaking_multiple_markers: !default.break_cfg.breaking_multiple_markers,
                breaking_start_marker: !default.break_cfg.breaking_start_marker,
            },
        };

        let parsed = "keep-inline-html, keep-footnotes , modify-tasklists, modify-tables, \
            breaking-multiple-markers, breaking-start-marker"
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
