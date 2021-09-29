fixred
======
[![crate][crates-io-badge]][crates-io]
[![CI][ci-badge]][ci]

[fixred][repo] is a command line utility to fix outdated links in files with redirect URLs.

<img src="https://github.com/rhysd/ss/raw/master/fixred/main.gif" alt="demo" width="590" height="396" />

## Installation

fixred is installed via [cargo][] package manager. [libcurl][] is necessary as dependency.

```sh
cargo install fixred
fixred --help
```

If you don't have Rust toolchain, [a Docker image][docker] is also available.

```sh
docker run -it --rm rhysd/fixred:latest --help
```

## Usage

fixred checks redirects of URLs in text files. When a URL is redirected, fixred replaces it with the redirected one.
fixred ignores invalid URLs or broken links (e.g. 404) to avoid false positives in extracted URLs.

See the help output for each flags, options, and arguments.

```sh
fixred --help
```

### Fix files

Most basic usage is fixing files by passing them to command line arguments.

```sh
# Fix a file
fixred ./docs/usage.md

# Fix all files in a directory (recursively)
fixred ./docs

# Multiple paths can be passed
fixred ./README.md ./CONTRIBUTING.md ./docs
```

### Fix stdin

When no argument is given, fixred reads stdin and outputs result to stdout.

```sh
cat ./docs/usage.md | fixred
```

### Run via Docker container

Mount local directories with `-v` and pass an environment variable (if necessary) with `-e`. Running
[the Docker image][docker] executes `fixred` executable by default.

```sh
# Fix all files in ./docs
docker run -it --rm -v $(pwd):/app -e FIXRED_LOG=info rhysd/fixred:latest /app/docs
```

Passing the input via stdin is also possible. The result is output to stdout.

```sh
# Fix stdin and output the result to stdout
cat ./docs/usage.md | docker run -i --rm rhysd/fixred:latest
```

### Redirect only once

By default, fixred follows redirects repeatedly and uses the last URL to replace. However, sometimes redirecting only
once would be more useful. `--shallow` flag is available for it.

For example, link to raw README file in `rhysd/vim-crystal` repository (moved to `vim-crystal/vim-crystal` later) is
redirected as follows.

1. https://github.com/rhysd/vim-crystal/raw/master/README.md
2. https://github.com/vim-crystal/vim-crystal/raw/master/README.md
3. https://raw.githubusercontent.com/vim-crystal/vim-crystal/master/README.md

When you want to fix 1. to 2. but don't want to fix 1. to 3., `--shallow` is useful.

```sh
fixred --shallow ./README.md
```

### Filtering URLs

When you want to fix only specific links in a file, filtering URLs with regular expressions is available. The following
example fixes URLs which starts with `https://github.com/` using `--extract` option.

```sh
fixred --extract '^https://github\.com/' ./docs
```

`--ignore` option is an invert version of `--extract`. URLs matched to the pattern are ignored. The following example
avoids to resolve URLs which contain hashes.

```sh
fixred --ignore '#' ./docs
```

### Verbose logs

By default, fixred outputs nothing when it runs successfully. For verbose log outputs, `--verbose` flag or `$FIXRED_LOG`i
environment variable is available.

```sh
# Outputs which file is being processed
fixred --verbose
# Or
FIXRED_LOG=info fixred ./docs

# Outputs what fixred is doing in detail
FIXRED_LOG=debug fixred ./docs
```

### Real-world example

- https://github.com/rhysd/actionlint/commit/0b7375279d2caf63203701eccc39ab091cc83a50
- https://github.com/rhysd/actionlint/commit/a3f270b313affa81cc41709587cbd2588d4ac4ce

Here is an example of usage in [actionlint][] project.

## Use this tool as Rust library

Please see [the API document][api]. And for the real world example, please see [src](./src) directory.

To install as dependency, add `fixred` to your `Cargo.toml` file. Ensure to disable default features.
It removes all unnecessary dependencies for using this tool as library.

```toml
[dependencies]
fixred = { version = "1", default-features = false, features = [] }
```

Here is a small example code

```rust
use fixred::resolve::CurlResolver;
use fixred::redirect::Redirector;

fn main() {
    let red = Redirector::<CurlResolver>::default();
    let fixed = red.fix(std::io::stdin(), std::io::stdout()).unwrap();
    eprintln!("Fixed {} link(s)", fixed);
}
```

## License

Distributed under [the MIT license](./LICENSE.txt).

[repo]: https://github.com/rhysd/fixred
[cargo]: https://doc.rust-lang.org/cargo/
[libcurl]: https://curl.se/libcurl/
[ci]: https://github.com/rhysd/fixred/actions/workflows/ci.yaml
[ci-badge]: https://github.com/rhysd/fixred/actions/workflows/ci.yaml/badge.svg
[crates-io]: https://crates.io/crates/fixred
[crates-io-badge]: https://img.shields.io/crates/v/fixred.svg
[actionlint]: https://github.com/rhysd/actionlint
[docker]: https://hub.docker.com/r/rhysd/fixred
[api]: https://docs.rs/crate/fixred
