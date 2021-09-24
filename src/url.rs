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

pub struct UrlFinder {
    ac: AhoCorasick,
}

impl UrlFinder {
    pub fn new() -> Self {
        let ac = AhoCorasick::new(&["https://", "http://"]);
        Self { ac }
    }

    pub fn find_all(&self, content: &str) -> Vec<(usize, usize)> {
        self.ac
            .find_iter(content)
            .map(|m| {
                let start = m.start();
                let end = m.end();

                let mut saw_term = false;
                let mut idx = 0;
                for (i, c) in content[end..].char_indices() {
                    if saw_term {
                        idx = i;
                        saw_term = false;
                    }
                    match url_char_kind(c) {
                        Char::NonTerm => {}
                        Char::Term => {
                            idx = i;
                            saw_term = true;
                        }
                        Char::Invalid => break,
                    }
                }
                let end = end + idx;
                (start, end)
            })
            .collect()
    }
}
