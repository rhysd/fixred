use anyhow::Result;
use chashmap::CHashMap;
use curl::easy::Easy;
use log::debug;
use regex::Regex;

pub trait Resolver: Default + Sync {
    fn select(&mut self, r: Option<Regex>);
    fn reject(&mut self, r: Option<Regex>);
    fn resolve(&self, url: &str) -> Result<Option<String>>;
}

pub struct CurlResolver {
    select: Option<Regex>,
    reject: Option<Regex>,
    cache: CHashMap<String, Option<String>>,
}

impl CurlResolver {
    fn should_redirect(&self, url: &str) -> bool {
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
}

impl Default for CurlResolver {
    fn default() -> Self {
        Self {
            select: None,
            reject: None,
            cache: CHashMap::new(),
        }
    }
}

impl Resolver for CurlResolver {
    fn select(&mut self, r: Option<Regex>) {
        self.select = r;
    }

    fn reject(&mut self, r: Option<Regex>) {
        self.reject = r;
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
}
