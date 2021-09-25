use aho_corasick::AhoCorasick;

enum Char {
    Invalid,
    Term,
    NonTerm,
}

// https://datatracker.ietf.org/doc/html/rfc3986#section-2
// > unreserved  = ALPHA / DIGIT / "-" / "." / "_" / "~"
// > reserved    = gen-delims / sub-delims
// > gen-delims  = ":" / "/" / "?" / "#" / "[" / "]" / "@"
// > sub-delims  = "!" / "$" / "&" / "'" / "(" / ")" / "*" / "+" / "," / ";" / "="
fn url_char_kind(c: char) -> Char {
    match c {
        c if c.is_alphanumeric() => Char::Term,
        '.' | ':' | '?' | '#' | '[' | ']' | '@' | '!' | '$' | '&' | '\'' | '(' | ')' | '*'
        | '+' | ',' | ';' => Char::NonTerm,
        '-' | '_' | '~' | '/' | '=' => Char::Term,
        _ => Char::Invalid,
    }
}

pub fn find_all_urls(content: &str) -> Vec<(usize, usize)> {
    AhoCorasick::new(&["https://", "http://"])
        .find_iter(content)
        .map(|m| {
            let start = m.start();
            let end = m.end();

            let mut idx = 0;
            for (i, c) in content[end..].char_indices() {
                match url_char_kind(c) {
                    Char::NonTerm => {}
                    Char::Term => {
                        // Since range is [start, end), idx should be index of the next character
                        idx = i + c.len_utf8();
                    }
                    Char::Invalid => break,
                }
            }
            let end = end + idx;
            (start, end)
        })
        .collect()
}
