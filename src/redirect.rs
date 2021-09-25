use crate::replace::{replace_all, Replacement};
use crate::resolve::{CurlResolver, Resolver};
use crate::url::UrlFinder;
use anyhow::Result;
use log::{debug, info};
use rayon::prelude::*;
use regex::Regex;
use std::ffi::OsStr;
use std::fs;
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Default)]
pub struct Redirector<R: Resolver> {
    resolver: R,
}

impl<R: Resolver> Redirector<R> {
    pub fn extract(mut self, r: Option<Regex>) -> Self {
        debug!("Regex to extract URLs: {:?}", r);
        self.resolver.extract(r);
        self
    }

    pub fn ignore(mut self, r: Option<Regex>) -> Self {
        debug!("Regex to ignore URLs: {:?}", r);
        self.resolver.ignore(r);
        self
    }

    pub fn shallow(mut self, b: bool) -> Self {
        debug!("Shallow redirect?: {}", b);
        self.resolver.shallow(b);
        self
    }

    fn find_and_replace<W: Write>(&self, out: W, content: &str) -> Result<usize> {
        let spans = UrlFinder::new().find_all(content); // Collect to Vec to use par_iter which is more efficient than par_bridge
        debug!("Found {} links", spans.len());
        let replacements = spans
            .into_par_iter()
            .filter_map(
                |(start, end)| match self.resolver.resolve(&content[start..end]) {
                    Ok(u) => u.map(|text| Ok(Replacement { start, end, text })),
                    Err(e) => Some(Err(e)),
                },
            )
            .collect::<Result<Vec<_>>>()?; // Collect to Vec to check errors before overwriting files
        debug!("Found {} redirects", replacements.len());
        replace_all(out, content, &replacements)?;
        Ok(replacements.len())
    }

    pub fn fix_file(&self, file: PathBuf) -> Result<()> {
        info!("Fixing redirects in {:?}", &file);

        let content = fs::read_to_string(&file)?;
        let mut out = BufWriter::new(fs::File::create(&file)?);
        let count = self.find_and_replace(&mut out, &content)?;
        out.flush()?;

        info!("Fixed {} links in {:?}", count, &file);
        Ok(())
    }

    pub fn fix_all_files<'a>(&self, paths: impl Iterator<Item = &'a OsStr>) -> Result<usize> {
        let count = paths
            .flat_map(WalkDir::new)
            .filter(|e| match e {
                Ok(e) => e.metadata().map(|m| m.is_file()).unwrap_or(false),
                Err(_) => true,
            })
            .try_fold(0, |c, e| self.fix_file(e?.into_path()).map(|_| c + 1))?;
        Ok(count)
    }

    pub fn fix<T: Read, U: Write>(&self, mut reader: T, writer: U) -> Result<usize> {
        let mut content = String::new();
        reader.read_to_string(&mut content)?;
        let content = &content;
        self.find_and_replace(writer, content)
    }
}

pub type CurlRedirector = Redirector<CurlResolver>;
