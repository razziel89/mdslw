// This build script downloads the language files used by mdslw.

use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

const LANG_SUPPRESSION_URL: &str = "https://raw.githubusercontent.com/unicode-org/cldr-json/main/cldr-json/cldr-segments-full/segments";

fn main() {
    println!("cargo::rerun-if-changed=src/lang.rs");
    println!("cargo::rerun-if-changed=src/lang/ac");
    println!("cargo::rerun-if-changed=src/cfg.rs");
    // Extract supported language names from a comment line like this in cfg.rs:
    // // Supported languages are: de en es fr it. Use "ac" for "author's choice",{n}   a list
    let cfg_file_path = PathBuf::from_str("src/cfg.rs").expect("failed to build path to cfg.rs");
    let cfg_file = fs::read(cfg_file_path).expect("failed to read cfg.rs");
    let cfg_file_content = String::from_utf8_lossy(&cfg_file);
    let langs = cfg_file_content
        // Extract the line that contains the content.
        .split('\n')
        .filter(|line| line.contains("Supported languages are:"))
        .take(1)
        // Extract the bit between the first colon and the first full stop.
        .flat_map(|line| line.split([':', '.']))
        .skip(1)
        .take(1)
        // Split into words. Those are the languages we support.
        .flat_map(|spec| spec.split_whitespace())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();

    assert!(!langs.is_empty(), "langs must not be empty");
    // Retrieve the list of keep words according to unicode. Also make sure each file ends on a line
    // break.
    let base_path =
        Path::new(&env::var_os("OUT_DIR").expect("OUT_DIR env var must be set")).join("lang");
    if let Ok(true) = fs::exists(&base_path) {
        fs::remove_dir_all(&base_path).expect("failed to remove outdated lang spec download dir");
    }
    fs::create_dir(&base_path).expect("failed to create lang spec download dir");
    // Copy the special "ac" specs over.
    let ac_path = PathBuf::from_str("src/lang/ac").expect("failed to build path to lang spec ac");
    fs::copy(ac_path, base_path.join("ac")).expect("failed to copy ac lang specs");
    // At compile time, we override the path used to import the language specs from. They will be
    // incorporated into the executable.
    println!(
        "cargo::rustc-env=MDSLW_LANG_DIR={}",
        base_path.to_string_lossy()
    );
    // Download and extract for all other languages.
    for lang in langs {
        eprintln!("building suppressions for language {}", lang);
        let data = reqwest::blocking::get(format!(
            "{}/{}/suppressions.json",
            LANG_SUPPRESSION_URL, lang
        ))
        .expect("downloading language")
        .json::<Value>()
        .expect("parsing response as json");

        // Get words from here:
        // .segments.segmentations.SentenceBreak.standard[].suppression
        // We do this extraction only once here. Thus, we do mot build a lot of nested custom types
        // to parse this single JSON payload. Instead, we use the facilities provided by the
        // json_serde::Value type to perform this one-time extraction.
        let words = data
            .as_object()
            .expect("json .")
            .get("segments")
            .expect("json .segments")
            .as_object()
            .expect("json .segments.")
            .get("segmentations")
            .expect("json .segments.segmentations")
            .as_object()
            .expect("json .segments.segmentations.")
            .get("SentenceBreak")
            .expect("json .segments.segmentations.SentenceBreak")
            .as_object()
            .expect("json .segments.segmentations.SentenceBreak.")
            .get("standard")
            .expect("json .segments.segmentations.SentenceBreak.standard")
            .as_array()
            .expect("json .segments.segmentations.SentenceBreak.standard[]")
            .iter()
            .map(|val| {
                val.as_object()
                    .expect("json .segments.segmentations.SentenceBreak.standard[].")
                    .get("suppression")
                    .expect("json .segments.segmentations.SentenceBreak.standard[].suppression")
                    .as_str()
                    .expect(".segments.segmentations.SentenceBreak.standard[].suppression.")
            })
            .collect::<Vec<_>>();

        assert!(!words.is_empty(), "words must not be empty");
        fs::write(base_path.join(lang), words.join("\n") + "\n").expect("writing file to disk");
    }
}
