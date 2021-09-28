use aho_corasick::AhoCorasick;

enum Char {
    Invalid,
    Term,
    NonTerm,
}

// https://datatracker.ietf.org/doc/html/rfc3986#section-2
// > unreserved   = ALPHA / DIGIT / "-" / "." / "_" / "~"
// > reserved     = gen-delims / sub-delims
// > gen-delims   = ":" / "/" / "?" / "#" / "[" / "]" / "@"
// > sub-delims   = "!" / "$" / "&" / "'" / "(" / ")" / "*" / "+" / "," / ";" / "="
// > pct-encoded = "%" HEXDIG HEXDIG
fn url_char_kind(c: char) -> Char {
    match c {
        c if c.is_alphanumeric() => Char::Term,
        '.' | ':' | '?' | '#' | '[' | ']' | '@' | '!' | '$' | '&' | '\'' | '(' | ')' | '*'
        | '+' | ',' | ';' | '%' => Char::NonTerm,
        '-' | '_' | '~' | '/' | '=' => Char::Term,
        _ => Char::Invalid,
    }
}

pub fn find_all_urls(content: &str) -> Vec<(usize, usize)> {
    AhoCorasick::new(&["https://", "http://"])
        .find_iter(content)
        .filter_map(|m| {
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
            if idx == 0 {
                None
            } else {
                let end = end + idx;
                Some((start, end))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let v = find_all_urls("");
        assert!(v.is_empty());
    }

    #[test]
    fn no_url() {
        let v = find_all_urls("foo bar baz");
        assert!(v.is_empty());
    }

    #[test]
    fn empty_after_scheme() {
        let v = find_all_urls("contains(s, 'https://')");
        assert!(v.is_empty());
    }

    #[test]
    fn entire_url() {
        let s = "http://example.com";
        assert_eq!(find_all_urls(s), &[(0, s.len())]);

        let s = "https://example.com";
        assert_eq!(find_all_urls(s), &[(0, s.len())]);
    }

    #[test]
    fn url_in_sentence() {
        let s = "the URL is https://example.com.";
        let (b, e) = find_all_urls(s)[0];
        assert_eq!(&s[b..e], "https://example.com");

        let s = "the URL is https://example.com!";
        let (b, e) = find_all_urls(s)[0];
        assert_eq!(&s[b..e], "https://example.com");

        let s = "the URL is [the link](https://example.com)";
        let (b, e) = find_all_urls(s)[0];
        assert_eq!(&s[b..e], "https://example.com");
    }

    #[test]
    fn url_ends_with_slash() {
        let s = "the GitHub URL is https://github.com/, check it out";
        let (b, e) = find_all_urls(s)[0];
        assert_eq!(&s[b..e], "https://github.com/");
    }

    #[test]
    fn percent_encoding() {
        let s = "https://example.com/?foo=%E3%81%82%E3%81%84%E3%81%86%E3%81%88%E3%81%8A&bar=true";
        let t = format!("see the URL {} for more details", s);
        let (b, e) = find_all_urls(&t)[0];
        assert_eq!(&t[b..e], s);
    }

    #[test]
    fn multiple_urls() {
        let s = "
            - Repository: https://github.com/rhysd/actionlint
            - Playground: https://rhysd.github.io/actionlint/
            - GitHub Actions official documentations
              - Workflow syntax: https://docs.github.com/en/actions/reference/workflow-syntax-for-github-actions
              - Expression syntax: https://docs.github.com/en/actions/reference/context-and-expression-syntax-for-github-actions
              - Built-in functions: https://docs.github.com/en/actions/reference/context-and-expression-syntax-for-github-actions#functions
              - Webhook events: https://docs.github.com/en/actions/reference/events-that-trigger-workflows#webhook-events
              - Self-hosted runner: https://docs.github.com/en/actions/hosting-your-own-runners/about-self-hosted-runners
              - Security: https://docs.github.com/en/actions/learn-github-actions/security-hardening-for-github-actions
            - CRON syntax: https://pubs.opengroup.org/onlinepubs/9699919799/utilities/crontab.html#tag_20_25_07
            - shellcheck: https://github.com/koalaman/shellcheck
            - pyflakes: https://github.com/PyCQA/pyflakes
            - Japanese blog posts
              - GitHub Actions のワークフローをチェックする actionlint をつくった: https://rhysd.hatenablog.com/entry/2021/07/11/214313
              - actionlint v1.4 → v1.6 で実装した新機能の紹介: https://rhysd.hatenablog.com/entry/2021/08/11/221044
        ";

        let want = &[
            "https://github.com/rhysd/actionlint",
            "https://rhysd.github.io/actionlint/",
            "https://docs.github.com/en/actions/reference/workflow-syntax-for-github-actions",
            "https://docs.github.com/en/actions/reference/context-and-expression-syntax-for-github-actions",
            "https://docs.github.com/en/actions/reference/context-and-expression-syntax-for-github-actions#functions",
            "https://docs.github.com/en/actions/reference/events-that-trigger-workflows#webhook-events",
            "https://docs.github.com/en/actions/hosting-your-own-runners/about-self-hosted-runners",
            "https://docs.github.com/en/actions/learn-github-actions/security-hardening-for-github-actions",
            "https://pubs.opengroup.org/onlinepubs/9699919799/utilities/crontab.html#tag_20_25_07",
            "https://github.com/koalaman/shellcheck",
            "https://github.com/PyCQA/pyflakes",
            "https://rhysd.hatenablog.com/entry/2021/07/11/214313",
            "https://rhysd.hatenablog.com/entry/2021/08/11/221044",
        ];

        let have: Vec<_> = find_all_urls(s)
            .into_iter()
            .map(|(b, e)| &s[b..e])
            .collect();

        assert_eq!(have, want);
    }
}
