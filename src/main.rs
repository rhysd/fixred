use aho_corasick::AhoCorasick;
use anyhow::Result;
use chashmap::CHashMap;
use clap::{App, Arg};
use curl::easy::Easy;
use rayon::iter::ParallelBridge;
use rayon::prelude::*;
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use walkdir::WalkDir;

struct Replacement(usize, usize, String);

fn replace_all<W: Write>(mut out: W, input: &str, replacements: Vec<Replacement>) -> Result<()> {
    let mut i = 0;
    for replacement in replacements.into_iter() {
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
        println!("start!: {:?}", url);
        if let Some(u) = self.cache.get(url) {
            println!("cache!: {:?} -> {:?}", url, u);
            return Ok(u.clone());
        }

        let mut curl = Easy::new();
        curl.url(url)?;
        curl.perform()?;
        let red = curl.redirect_url()?.map(str::to_string);
        println!("res: {} -> {:?}", url, red);
        self.cache.insert(url.to_string(), red.clone());
        Ok(red)
    }

    fn redirect(&self, file: PathBuf) -> Result<()> {
        let content = fs::read_to_string(&file)?;

        let ac = AhoCorasick::new(&["https://", "http://"]);
        let replacements = ac
            .find_iter(&content)
            .filter_map(|m| {
                let start = m.start();
                let mut end = m.end();

                for (i, c) in content[end..].char_indices() {
                    match url_char_kind(c) {
                        Char::NonTerm => {}
                        Char::Term => end = i,
                        Char::Invalid => break,
                    }
                }

                match self.resolve(&content[start..end]) {
                    Ok(u) => u.map(|u| Ok(Replacement(start, end, u))),
                    Err(e) => Some(Err(e)),
                }
            })
            .collect::<Result<Vec<_>>>()?;

        let out = fs::File::create(&file)?;
        replace_all(out, &content, replacements)
    }

    fn redirect_all<'a>(&self, paths: impl Iterator<Item = &'a OsStr> + Send) -> Result<()> {
        paths
            .flat_map(WalkDir::new)
            .filter(|e| match e {
                Ok(e) => e.metadata().map(|m| m.is_file()).unwrap_or(false),
                Err(_) => true,
            })
            .map(|e| Result::<_>::Ok(e?.into_path()))
            .par_bridge()
            .map(|p| self.redirect(p?))
            .collect::<Result<()>>()
    }
}

fn main() -> Result<()> {
    let matches = App::new("fixred")
        .arg(
            Arg::new("PATH")
                .about("Directory or file to fix")
                .multiple_values(true),
        )
        .get_matches();
    if let Some(paths) = matches.values_of_os("PATH") {
        let red = Redirector::default();
        red.redirect_all(paths)
    } else {
        unimplemented!("TODO: read stdin and output result to stdout")
    }
}
