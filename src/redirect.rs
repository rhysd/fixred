use crate::replace::{replace_all, Replacement};
use crate::resolve::Resolver;
use crate::url::UrlFinder;
use anyhow::Result;
use chashmap::CHashMap;
use log::{debug, info};
use rayon::prelude::*;
use regex::Regex;
use std::ffi::OsStr;
use std::fs;
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;
use walkdir::WalkDir;

pub struct Redirector {
    select: Option<Regex>,
    reject: Option<Regex>,
    cache: CHashMap<String, Option<String>>,
}

impl Default for Redirector {
    fn default() -> Self {
        Self {
            select: None,
            reject: None,
            cache: CHashMap::new(),
        }
    }
}

impl Redirector {
    pub fn select(mut self, r: Option<Regex>) -> Self {
        debug!("Regex to select URLs: {:?}", r);
        self.select = r;
        self
    }

    pub fn reject(mut self, r: Option<Regex>) -> Self {
        debug!("Regex to reject URLs: {:?}", r);
        self.reject = r;
        self
    }

    fn is_match_url(&self, url: &str) -> bool {
        if let Some(r) = &self.select {
            if !r.is_match(url) {
                return false;
            }
        }
        if let Some(r) = &self.reject {
            if r.is_match(url) {
                return false;
            }
        }
        true
    }

    fn resolve<R: Resolver>(&self, url: &str) -> Result<Option<String>> {
        debug!("Resolving {}", url);
        if let Some(u) = self.cache.get(url) {
            debug!("Cache hit: {} -> {:?}", url, *u);
            return Ok(u.clone());
        }

        let mut res = R::default();
        let red = res
            .resolve(url)?
            .and_then(|u| (u != url && self.is_match_url(u)).then(|| u.to_string()));
        debug!("Resolved redirect: {} -> {:?}", url, red);
        self.cache.insert(url.to_string(), red.clone());
        Ok(red)
    }

    fn find_and_replace<W: Write, R: Resolver>(&self, out: W, content: &str) -> Result<usize> {
        let spans = UrlFinder::new().find_all(content); // Collect to Vec to use par_iter which is more efficient than par_bridge
        debug!("Found {} links", spans.len());
        let replacements = spans
            .into_par_iter()
            .filter_map(
                |(start, end)| match self.resolve::<R>(&content[start..end]) {
                    Ok(u) => u.map(|text| Ok(Replacement { start, end, text })),
                    Err(e) => Some(Err(e)),
                },
            )
            .collect::<Result<Vec<_>>>()?; // Collect to Vec to check errors before overwriting files
        let len = replacements.len();
        replace_all(out, content, replacements.into_iter())?;
        Ok(len)
    }

    pub fn fix_file<R: Resolver>(&self, file: PathBuf) -> Result<()> {
        info!("Fixing redirects in {:?}", &file);

        let content = fs::read_to_string(&file)?;
        let out = fs::File::create(&file)?;
        let out = BufWriter::new(out);
        let count = self.find_and_replace::<_, R>(out, &content)?;

        info!("Fixed {} links in {:?}", count, &file);
        Ok(())
    }

    pub fn fix_all_files<'a, I, R>(&self, paths: I) -> Result<()>
    where
        R: Resolver,
        I: Iterator<Item = &'a OsStr> + Send,
    {
        let count = paths
            .flat_map(WalkDir::new)
            .filter(|e| match e {
                Ok(e) => e.metadata().map(|m| m.is_file()).unwrap_or(false),
                Err(_) => true,
            })
            .try_fold(0, |c, e| self.fix_file::<R>(e?.into_path()).map(|_| c + 1))?;
        info!("Processed {} files", count);
        Ok(())
    }

    pub fn fix<T: Read, U: Write, R: Resolver>(&self, mut reader: T, writer: U) -> Result<usize> {
        let mut content = String::new();
        reader.read_to_string(&mut content)?;
        let content = &content;
        self.find_and_replace::<_, R>(writer, content)
    }
}
