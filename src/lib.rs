//! This is a library part of [fixred][repo] tool.
//!
//! To install as dependency, add `fixred` to your `Cargo.toml` file. Ensure to disable default features.
//! It removes all unnecessary dependencies for using this tool as library.
//!
//! ```toml
//! [dependencies]
//! fixred = { version = "1", default-features = false, features = [] }
//! ```
//!
//! Here is a small example code.
//!
//! ```
//! use fixred::resolve::CurlResolver;
//! use fixred::redirect::Redirector;
//!
//! let red = Redirector::<CurlResolver>::default();
//! let fixed = red.fix(std::io::stdin(), std::io::stdout()).unwrap();
//! eprintln!("Fixed {} link(s)", fixed);
//! ```
//!
//! For the real world example, please see [src][] directory.
//!
//! [repo]: https://github.com/rhysd/fixred
//! [src]: https://github.com/rhysd/fixred/tree/main/src

pub mod redirect;
pub mod replace;
pub mod resolve;
pub mod url;

#[cfg(test)]
mod test_helper;
