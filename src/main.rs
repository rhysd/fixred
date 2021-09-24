use aho_corasick::AhoCorasick;
use anyhow::Result;
use chashmap::CHashMap;
use clap::{App, Arg};
use curl::easy::Easy;
use log::{debug, info};
use rayon::prelude::*;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;
use walkdir::WalkDir;

struct Replacement(usize, usize, String);

fn replace_all<W: Write>(
    mut out: W,
    input: &str,
    replacements: impl Iterator<Item = Replacement>,
) -> Result<()> {
    let mut i = 0;
    for replacement in replacements {
        let Replacement(s, e, url) = replacement;
        out.write_all(input[i..s].as_bytes())?;
        out.write_all(url.as_bytes())?;
        i = e;
    }
    out.write_all(input[i..].as_bytes())?;
    Ok(())
}

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

struct UrlFinder {
    ac: AhoCorasick,
}

impl UrlFinder {
    fn new() -> Self {
        let ac = AhoCorasick::new(&["https://", "http://"]);
        Self { ac }
    }

    fn find_all(&self, content: &str) -> Vec<(usize, usize)> {
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

struct Redirector {
    cache: CHashMap<String, Option<String>>,
}

impl Default for Redirector {
    fn default() -> Self {
        Self {
            cache: CHashMap::new(),
        }
    }
}

impl Redirector {
    fn resolve(&self, url: impl AsRef<str>) -> Result<Option<String>> {
        let url = url.as_ref();
        debug!("Resolving {}", url);
        if let Some(u) = self.cache.get(url) {
            debug!("Cache hit: {} -> {:?}", url, *u);
            return Ok(u.clone());
        }

        let mut curl = Easy::new();
        curl.follow_location(true)?;
        curl.url(url)?;
        curl.perform()?;
        let red = curl
            .effective_url()?
            .and_then(|u| (u != url).then(|| u.to_string()));
        debug!("Resolved redirect: {} -> {:?}", url, red);
        self.cache.insert(url.to_string(), red.clone());
        Ok(red)
    }

    fn find_and_replace<W: Write>(&self, out: W, content: &str) -> Result<usize> {
        let spans = UrlFinder::new().find_all(content); // Collect to Vec to use par_iter which is more efficient than par_bridge
        debug!("Found {} links", spans.len());
        let replacements = spans
            .into_par_iter()
            .filter_map(|(start, end)| match self.resolve(&content[start..end]) {
                Ok(u) => u.map(|u| Ok(Replacement(start, end, u))),
                Err(e) => Some(Err(e)),
            })
            .collect::<Result<Vec<_>>>()?; // Collect to Vec to check errors before overwriting files
        let len = replacements.len();
        replace_all(out, content, replacements.into_iter())?;
        Ok(len)
    }

    fn fix_file(&self, file: PathBuf) -> Result<()> {
        info!("Fixing redirects in {:?}", &file);

        let content = fs::read_to_string(&file)?;
        let out = fs::File::create(&file)?;
        let out = BufWriter::new(out);
        let count = self.find_and_replace(out, &content)?;

        info!("Fixed {} links in {:?}", count, &file);
        Ok(())
    }

    fn fix_all_files<'a>(&self, paths: impl Iterator<Item = &'a OsStr> + Send) -> Result<()> {
        let count = paths
            .flat_map(WalkDir::new)
            .filter(|e| match e {
                Ok(e) => e.metadata().map(|m| m.is_file()).unwrap_or(false),
                Err(_) => true,
            })
            .try_fold(0, |c, e| self.fix_file(e?.into_path()).map(|_| c + 1))?;
        info!("Processed {} files", count);
        Ok(())
    }

    fn fix<R: Read, W: Write>(&self, mut reader: R, writer: W) -> Result<usize> {
        let mut content = String::new();
        reader.read_to_string(&mut content)?;
        let content = &content;
        self.find_and_replace(writer, content)
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let matches = App::new("fixred")
        .arg(
            Arg::new("PATH")
                .about("Directory or file path to fix")
                .multiple_values(true),
        )
        .get_matches();

    let red = Redirector::default();
    if let Some(paths) = matches.values_of_os("PATH") {
        debug!("Some paths are given via arguments");
        red.fix_all_files(paths)
    } else {
        info!("Fixing redirects in stdin");
        let stdin = io::stdin();
        let stdout = io::stdout();
        let count = red.fix(stdin.lock(), stdout.lock())?;
        info!("Fixed {} links in stdin", count);
        Ok(())
    }
}
