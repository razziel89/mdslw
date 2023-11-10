use core::ops::Range;
use pulldown_cmark::{Event, Parser, Tag};
use std::collections::HashSet;

fn extract_link_text_range(range: Range<usize>, text: &str) -> Option<Range<usize>> {
    let first = range.start;
    let snippet = &text[range];
    let start = snippet.find('[');
    let end = snippet.find(']');
    if let (Some(start), Some(end)) = (start, end) {
        let result = first + start..first + end + 1;
        eprintln!(
            "{:?} {:?} {} {}",
            result.start,
            result.end,
            snippet,
            &text[result.clone()]
        );
        Some(result)
    } else {
        None
    }
}

pub fn replace_spaces_in_links_by_nbsp(text: String) -> String {
    let char_indices_in_links = Parser::new(&text)
        .into_offset_iter()
        .filter_map(|(event, range)| match event {
            Event::Start(Tag::Link(..)) => extract_link_text_range(range, &text),
            _ => None,
        })
        .flatten()
        .collect::<HashSet<_>>();

    text.char_indices()
        .map(|(idx, ch)| {
            if ch.is_whitespace() && char_indices_in_links.contains(&idx) {
                '\u{00a0}'
            } else {
                ch
            }
        })
        .collect::<String>()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn replacing_spaces_only_in_links() {
        let original = "Outside of link, [inside of link](http://some-url), again outside.";
        let expected = "Outside of link, [inside of link](http://some-url), again outside.";

        let replaced = replace_spaces_in_links_by_nbsp(original.to_string());

        assert_eq!(replaced, expected);
    }
}
