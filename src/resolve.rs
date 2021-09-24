use anyhow::Result;
use curl::easy::Easy;

pub trait Resolver: Default {
    fn resolve<'s>(&'s mut self, url: &str) -> Result<Option<&'s str>>;
}

pub struct CurlResolver(Easy);

impl Default for CurlResolver {
    fn default() -> Self {
        Self(Easy::new())
    }
}

impl Resolver for CurlResolver {
    fn resolve<'s>(&'s mut self, url: &str) -> Result<Option<&'s str>> {
        self.0.follow_location(true)?;
        self.0.url(url)?;
        self.0.perform()?;
        Ok(self.0.effective_url()?)
    }
}
