use anyhow::Result;
use chashmap::CHashMap;
use curl::easy::Easy;
use log::debug;
use regex::Regex;

pub trait Resolver: Default + Sync {
    fn extract(&mut self, r: Option<Regex>);
    fn ignore(&mut self, r: Option<Regex>);
    fn shallow(&mut self, b: bool);
    fn resolve(&self, url: &str) -> Result<Option<String>>;
}

#[derive(Default)]
pub struct CurlResolver {
    extract: Option<Regex>,
    ignore: Option<Regex>,
    shallow: bool,
    cache: CHashMap<String, Option<String>>,
}

impl CurlResolver {
    fn should_redirect(&self, url: &str) -> bool {
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
}

impl Resolver for CurlResolver {
    fn extract(&mut self, r: Option<Regex>) {
        self.extract = r;
    }

    fn ignore(&mut self, r: Option<Regex>) {
        self.ignore = r;
    }

    fn shallow(&mut self, b: bool) {
        self.shallow = b;
    }

    fn resolve(&self, url: &str) -> Result<Option<String>> {
        debug!("Resolving {}", url);

        if let Some(u) = self.cache.get(url) {
            debug!("Cache hit: {} -> {:?}", url, *u);
            return Ok(u.clone());
        }

        if !self.should_redirect(url) {
            debug!("Skipped URL: {}", url);
            self.cache.insert(url.to_string(), None);
            return Ok(None);
        }

        debug!("Sending HEAD request to {}", url);
        let mut curl = Easy::new();
        curl.nobody(true)?;
        curl.url(url)?;
        let resolved = if self.shallow {
            curl.perform()?;
            curl.redirect_url()? // Get the first redirect URL
        } else {
            curl.follow_location(true)?;
            curl.perform()?;
            curl.effective_url()?
        };
        let red = resolved.and_then(|u| (u != url).then(|| u.to_string()));
        debug!("Resolved redirect: {} -> {:?}", url, red);
        self.cache.insert(url.to_string(), red.clone());
        Ok(red)
    }
}
