use anyhow::Result;
use chashmap::CHashMap;
use curl::easy::Easy;
use log::{debug, warn};

pub trait Resolver: Default + Sync {
    fn shallow(&mut self, b: bool);
    fn resolve(&self, url: &str) -> Option<String>;
}

#[derive(Default)]
pub struct CurlResolver {
    shallow: bool,
    cache: CHashMap<String, Option<String>>,
}

impl CurlResolver {
    fn resolve_impl(&self, url: &str) -> Result<Option<String>> {
        debug!("Resolving {}", url);

        if let Some(u) = self.cache.get(url) {
            debug!("Cache hit: {} -> {:?}", url, *u);
            return Ok(u.clone());
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

impl Resolver for CurlResolver {
    fn shallow(&mut self, enabled: bool) {
        self.shallow = enabled;
    }

    fn resolve(&self, url: &str) -> Option<String> {
        // Do not return error on resolving URLs because it is normal case that broken URL is passed to this function.
        match self.resolve_impl(url) {
            Ok(ret) => ret,
            Err(err) => {
                warn!("Could not resolve {:?}: {}", url, err);
                None
            }
        }
    }
}
