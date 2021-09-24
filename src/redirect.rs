use crate::replace::{replace_all, Replacement};
use crate::url::UrlFinder;
use anyhow::Result;
use chashmap::CHashMap;
use curl::easy::Easy;
use log::{debug, info};
use rayon::prelude::*;
use std::ffi::OsStr;
use std::fs;
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;
use walkdir::WalkDir;

pub struct Redirector {
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
                Ok(u) => u.map(|text| Ok(Replacement { start, end, text })),
                Err(e) => Some(Err(e)),
            })
            .collect::<Result<Vec<_>>>()?; // Collect to Vec to check errors before overwriting files
        let len = replacements.len();
        replace_all(out, content, replacements.into_iter())?;
        Ok(len)
    }

    pub fn fix_file(&self, file: PathBuf) -> Result<()> {
        info!("Fixing redirects in {:?}", &file);

        let content = fs::read_to_string(&file)?;
        let out = fs::File::create(&file)?;
        let out = BufWriter::new(out);
        let count = self.find_and_replace(out, &content)?;

        info!("Fixed {} links in {:?}", count, &file);
        Ok(())
    }

    pub fn fix_all_files<'a>(&self, paths: impl Iterator<Item = &'a OsStr> + Send) -> Result<()> {
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

    pub fn fix<R: Read, W: Write>(&self, mut reader: R, writer: W) -> Result<usize> {
        let mut content = String::new();
        reader.read_to_string(&mut content)?;
        let content = &content;
        self.find_and_replace(writer, content)
    }
}
