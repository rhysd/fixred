fixred
======

[fixred][repo] is a command line utility to fix outdated links in files with redirect URLs.

## Installation

fixred is installed via [cargo][] package manager. [libcurl][] is necessary as dependency.

```sh
cargo install fixred
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

`--ignore` option is an invert version of `--extract`. URLs matched to the pattern are ignored.

### Verbose logs

By default, fixred outputs nothing when it runs successfully. For verbose log outputs, `$RUST_LOG` environment variable
is available.

```sh
# Outputs which file is being processed
RUST_LOG=info fixred ./docs

# Outputs what fixred is doing in detail
RUST_LOG=debug fixred ./docs
```

## License

Distributed under [the MIT license](./LICENSE.txt).

[repo]: https://github.com/rhysd/fixred
[cargo]: https://doc.rust-lang.org/cargo/
[libcurl]: https://curl.se/libcurl/
