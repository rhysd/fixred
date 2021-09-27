use anyhow::Result;
use chashmap::CHashMap;
use curl::easy::Easy;
use log::{debug, warn};
use url::Url;

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
    fn try_resolve(&self, url: &str) -> Result<Option<String>> {
        debug!("Resolving {}", url);

        if let Some(u) = self.cache.get(url) {
            debug!("Cache hit: {} -> {:?}", url, *u);
            return Ok(u.clone());
        }

        let parsed = Url::parse(url)?;

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
        let red = resolved.and_then(|u| {
            (u != url).then(|| {
                if let Some(fragment) = parsed.fragment() {
                    format!("{}#{}", u, fragment)
                } else {
                    u.to_string()
                }
            })
        });
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
        match self.try_resolve(url) {
            Ok(ret) => ret,
            Err(err) => {
                warn!("Could not resolve {:?}: {}", url, err);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_url_with_cache() {
        // Redirect: github.com/rhysd/ -> github.com/vim-crystal/ -> raw.githubusercontent
        let url = "https://github.com/rhysd/vim-crystal/raw/master/README.md";

        let res = CurlResolver::default();
        let resolved = res.try_resolve(url).unwrap();
        let resolved = resolved.unwrap();
        assert!(
            resolved.starts_with("https://raw.githubusercontent.com/vim-crystal/"),
            "URL: {}",
            resolved
        );

        assert_eq!(*res.cache.get(url).unwrap(), Some(resolved.clone()));

        let cached = res.try_resolve(url).unwrap();
        assert_eq!(resolved, cached.unwrap());
    }

    #[test]
    fn resolve_shallow_redirect() {
        // Redirect: github.com/rhysd/ -> github.com/vim-crystal/ -> raw.githubusercontent
        let url = "https://github.com/rhysd/vim-crystal/raw/master/README.md";

        let mut res = CurlResolver::default();
        res.shallow(true);
        let resolved = res.try_resolve(url).unwrap();
        let resolved = resolved.unwrap();
        assert!(
            resolved.starts_with("https://github.com/vim-crystal/vim-crystal/"),
            "URL: {}",
            resolved
        );
    }

    #[test]
    fn resolve_url_not_found() {
        // Redirect: github.com/rhysd/ -> github.com/vim-crystal/ -> raw.githubusercontent
        let url = "https://github.com/rhysd/this-repo-does-not-exist";

        let res = CurlResolver::default();
        let resolved = res.resolve(url);
        assert_eq!(resolved, None);

        assert_eq!(*res.cache.get(url).unwrap(), None);

        let cached = res.resolve(url);
        assert_eq!(resolved, cached);
    }

    #[test]
    fn resolve_url_with_fragment() {
        // Redirect: github.com/rhysd/ -> github.com/vim-crystal
        let url = "https://github.com/rhysd/vim-crystal#readme";

        let res = CurlResolver::default();
        let resolved = res.resolve(url).unwrap();
        assert!(resolved.ends_with("#readme"), "URL: {}", resolved);
    }

    #[test]
    fn url_parse_error() {
        let res = CurlResolver::default();
        let resolved = res.try_resolve("https://");
        assert!(resolved.is_err(), "{:?}", resolved);
    }
}
