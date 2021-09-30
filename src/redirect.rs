use crate::replace::{replace_all, Replacement};
use crate::resolve::{CurlResolver, Resolver};
use crate::url::find_all_urls;
use anyhow::{Context, Result};
use log::{debug, info, warn};
use rayon::prelude::*;
use regex::Regex;
use std::ffi::OsStr;
use std::fs;
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Default)]
pub struct Redirector<R: Resolver> {
    extract: Option<Regex>,
    ignore: Option<Regex>,
    resolver: R,
}

impl<R: Resolver> Redirector<R> {
    pub fn extract(mut self, pattern: Option<Regex>) -> Self {
        debug!("Regex to extract URLs: {:?}", pattern);
        self.extract = pattern;
        self
    }

    pub fn ignore(mut self, pattern: Option<Regex>) -> Self {
        debug!("Regex to ignore URLs: {:?}", pattern);
        self.ignore = pattern;
        self
    }

    pub fn shallow(mut self, enabled: bool) -> Self {
        debug!("Shallow redirect?: {}", enabled);
        self.resolver.shallow(enabled);
        self
    }

    fn should_resolve(&self, url: &str) -> bool {
        if let Some(r) = &self.extract {
            if !r.is_match(url) {
                return false;
            }
        }
        if let Some(r) = &self.ignore {
            if r.is_match(url) {
                return false;
            }
        }
        true
    }

    fn find_replacements(&self, content: &str) -> Vec<Replacement> {
        let spans = find_all_urls(content); // Collect to Vec to use par_iter which is more efficient than par_bridge
        debug!("Found {} links", spans.len());
        let replacements = spans
            .into_par_iter()
            .filter_map(|(start, end)| {
                let url = &content[start..end];
                if !self.should_resolve(url) {
                    debug!("Skipped URL: {}", url);
                    return None;
                }
                let url = self.resolver.resolve(url);
                url.map(|text| Replacement { start, end, text })
            })
            .collect::<Vec<_>>(); // Collect to Vec to check errors before overwriting files
        debug!("Found {} redirects", replacements.len());
        replacements
    }

    pub fn fix_file(&self, file: &Path) -> Result<()> {
        info!("Fixing redirects in {:?}", &file);

        let content = match fs::read_to_string(&file) {
            Err(err) => {
                warn!("Ignored non-UTF8 file {:?}: {}", &file, err);
                return Ok(());
            }
            Ok(s) => s,
        };
        let replacements = self.find_replacements(&content);
        if replacements.is_empty() {
            info!("Fixed no link in {:?} (skipped overwriting)", &file);
            return Ok(());
        }
        let mut out = BufWriter::new(fs::File::create(&file)?); // Truncate the file after all replacements are collected without error
        replace_all(&mut out, &content, &replacements)?;

        info!("Fixed {} links in {:?}", replacements.len(), &file);
        Ok(())
    }

    pub fn fix_all_files<'a>(&self, paths: impl Iterator<Item = &'a OsStr>) -> Result<usize> {
        paths
            .flat_map(WalkDir::new)
            .filter_map(|entry| match entry {
                Ok(entry) => match entry.metadata() {
                    Ok(m) if m.is_file() => Some(Ok(entry)),
                    Ok(_) => None,
                    Err(err) => Some(Err(err)),
                },
                Err(err) => Some(Err(err)),
            })
            .map(|entry| {
                let path = entry?.into_path();
                self.fix_file(&path)
                    .with_context(|| format!("While processing {:?}", &path))?;
                Ok(1)
            })
            .sum()
    }

    pub fn fix<T: Read, U: Write>(&self, mut input: T, output: U) -> Result<usize> {
        let mut content = String::new();
        input.read_to_string(&mut content)?;
        let content = &content;
        let replacements = self.find_replacements(content);
        replace_all(output, content, &replacements)?;
        Ok(replacements.len())
    }
}

pub type CurlRedirector = Redirector<CurlResolver>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helper::*;
    use std::iter;
    use std::path::PathBuf;

    type TestRedirector = Redirector<FooToPiyoResolver>;

    #[test]
    fn fix_all_files_recursively() {
        // Tests with actual file system

        let entries = &[
            TestDirEntry::File(
                "test1.txt",
                "https://foo1.example.com\nhttps://example.com/foo1\nhttps://example.com\n",
            ),
            TestDirEntry::Dir("dir1"),
            TestDirEntry::File(
                "dir1/test2.txt",
                "https://foo2.example.com\nhttps://example.com/foo2\nhttps://example.com\n",
            ),
            TestDirEntry::File(
                "dir1/test3.txt",
                "https://foo3.example.com\nhttps://example.com/foo3\nhttps://example.com\n",
            ),
            TestDirEntry::Dir("dir1/dir2"),
            TestDirEntry::File(
                "dir1/dir2/test4.txt",
                "https://foo4.example.com\nhttps://example.com/foo4\nhttps://example.com\n",
            ),
            TestDirEntry::File(
                "dir1/dir2/test5.txt",
                "https://foo5.example.com\nhttps://example.com/foo5\nhttps://example.com\n",
            ),
        ];

        let dir = TestDir::new(entries).unwrap();

        let red = TestRedirector::default();
        let root = &dir.root;
        let paths = &[root.join("test1.txt"), root.join("dir1")];
        let count = red.fix_all_files(paths.iter().map(|p| p.as_ref())).unwrap();
        assert_eq!(count, dir.files.len());

        let want: Vec<_> = dir
            .files
            .iter()
            .map(|(p, c)| (p.clone(), c.replace("foo", "piyo")))
            .collect();
        assert_files(&want);
    }

    #[test]
    fn ignore_non_utf8_file() {
        // Invalid UTF-8 sequence
        let content = b"\xf0\x28\x8c\xbc";
        std::str::from_utf8(content).unwrap_err();

        let entries = &[TestDirEntry::Binary("test.bin", content)];
        let dir = TestDir::new(entries).unwrap();

        let red = TestRedirector::default();
        let path = dir.root.join("test.bin");
        let count = red.fix_all_files(iter::once(path.as_ref())).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn read_file_error() {
        let red = TestRedirector::default();
        let mut p = PathBuf::new();
        p.push("this-file");
        p.push("does-not");
        p.push("exist.txt");
        red.fix_all_files(iter::once(p.as_ref())).unwrap_err();
    }

    #[test]
    fn fix_reader_writer() {
        let mut output = vec![];
        let input = "
            this is test https://foo1.example.com
            https://example.com/foo1
            https://example.com
            done.";

        let red = TestRedirector::default();
        let fixed = red.fix(input.as_bytes(), &mut output).unwrap();
        assert_eq!(fixed, 2);

        let want = input.replace("foo", "piyo");
        let have = String::from_utf8(output).unwrap();
        assert_eq!(want, have);
    }

    #[test]
    fn fix_shallow_redirect() {
        let mut output = vec![];
        let input = "
            this is test https://foo1.example.com
            https://example.com/foo1
            https://example.com
            done.";

        let red = TestRedirector::default().shallow(true);
        let fixed = red.fix(input.as_bytes(), &mut output).unwrap();
        assert_eq!(fixed, 2);

        let want = input.replace("foo", "bar");
        let have = String::from_utf8(output).unwrap();
        assert_eq!(want, have);
    }

    #[test]
    fn exract_urls() {
        let mut output = vec![];
        let input = "
            - https://github.com/rhysd/foo
            - https://rhysd.github.io/foo
            - https://docs.github.com/foo/some-docs
            - https://foo.example.com/foo
        ";

        let pat = Regex::new("github\\.com/").unwrap();
        let red = TestRedirector::default().extract(Some(pat));
        let fixed = red.fix(input.as_bytes(), &mut output).unwrap();
        assert_eq!(fixed, 2);

        let want = input
            .replace("github.com/rhysd/foo", "github.com/rhysd/piyo")
            .replace("docs.github.com/foo", "docs.github.com/piyo");
        let have = String::from_utf8(output).unwrap();
        assert_eq!(want, have);
    }

    #[test]
    fn ignore_urls() {
        let mut output = vec![];
        let input = "
            - https://github.com/rhysd/foo
            - https://rhysd.github.io/foo
            - https://docs.github.com/foo/some-docs
            - https://foo.example.com/foo
        ";

        let pat = Regex::new("github\\.com/").unwrap();
        let red = TestRedirector::default().ignore(Some(pat));
        let fixed = red.fix(input.as_bytes(), &mut output).unwrap();
        assert_eq!(fixed, 2);

        let want = input
            .replace("rhysd.github.io/foo", "rhysd.github.io/piyo")
            .replace("foo.example.com/foo", "piyo.example.com/piyo");
        let have = String::from_utf8(output).unwrap();
        assert_eq!(want, have);
    }

    #[test]
    fn extract_and_ignore_urls() {
        let mut output = vec![];
        let input = "
            - https://github.com/rhysd/foo
            - https://rhysd.github.io/foo
            - https://docs.github.com/foo/some-docs
            - https://foo.example.com/foo
        ";

        let pat1 = Regex::new("example\\.com/").unwrap();
        let pat2 = Regex::new("github\\.com/").unwrap();
        let red = TestRedirector::default()
            .extract(Some(pat1))
            .ignore(Some(pat2));
        let fixed = red.fix(input.as_bytes(), &mut output).unwrap();
        assert_eq!(fixed, 1);

        let want = input.replace("foo.example.com/foo", "piyo.example.com/piyo");
        let have = String::from_utf8(output).unwrap();
        assert_eq!(want, have);
    }
}
