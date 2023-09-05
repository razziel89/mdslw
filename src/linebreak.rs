use std::collections::HashSet;

pub fn insert_linebreaks_between_sentences(text: &str, indent: &str) -> String {
    let merged = merge_all_whitespace(text);
    let sentence_ends = find_sentence_ends(&merged);

    merged
        .char_indices()
        .filter_map(|(idx, el)| {
            if sentence_ends.contains(&Char::Skip(idx)) {
                None
            } else if sentence_ends.contains(&Char::Split(idx)) {
                Some(format!("\n{}{}", indent, el))
            } else {
                Some(format!("{}", el))
            }
        })
        .collect::<Vec<_>>()
        .join("")
        .trim_end()
        .to_string()
}

/// Replace all consecutive whitespace by a single space. That includes line breaks. This is like
/// piping through `tr -s '[:space:]' ' '` in the shell.
fn merge_all_whitespace(text: &str) -> String {
    let mut last_was_whitespace = false;

    text.chars()
        .filter_map(|el| {
            if el.is_whitespace() {
                if last_was_whitespace {
                    None
                } else {
                    last_was_whitespace = true;
                    Some(' ')
                }
            } else {
                last_was_whitespace = false;
                Some(el)
            }
        })
        .collect::<String>()
}

/// Check whether a character is one that may end a sentence and, thus, might warrant the addition
/// of a line break. This is hard-coded for now but can be outsourced into a config option.
fn is_sentence_end_marker(ch: char) -> bool {
    ch == '.' || ch == '!' || ch == '?' || ch == ':'
}

#[derive(Eq, Hash, PartialEq)]
enum Char {
    Skip(usize),
    Split(usize),
}

/// Check whether the last word is a special one that is knwon as an abbreviation because no line
/// break should be inserted after one. This is hard-coded so far but can be outsourced into a
/// config option.
fn is_keep_word(text: &Vec<char>, idx: usize) -> bool {
    // Check 4 character words.
    let word_4 = text[idx.checked_sub(3).unwrap_or(idx)..=idx]
        .into_iter()
        .collect::<String>();
    match word_4.as_str() {
        "etc." | "e.g." | "i.e." => {
            return true;
        }
        _ => {}
    }
    // Check 3 character words.
    let word_3 = text[idx.checked_sub(2).unwrap_or(idx)..=idx]
        .into_iter()
        .collect::<String>();
    match word_3.as_str() {
        "cf." => {
            return true;
        }
        _ => {}
    }
    false
}

fn find_sentence_ends(text: &str) -> HashSet<Char> {
    let lower = text
        .chars()
        .flat_map(|el| el.to_lowercase())
        .collect::<Vec<_>>();

    text.chars()
        .zip(text.chars().skip(1))
        .enumerate()
        .filter_map(|(idx, (first, second))| {
            let keep_word = is_keep_word(&lower, idx);
            if second.is_whitespace() && is_sentence_end_marker(first) && !keep_word {
                Some([Char::Skip(idx + 1), Char::Split(idx + 2)])
            } else {
                None
            }
        })
        .flatten()
        .collect::<HashSet<_>>()
}
