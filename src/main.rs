pub mod redirect;
pub mod replace;
pub mod resolve;
pub mod url;

use anyhow::{Context, Result};
use clap::{App, AppSettings, Arg};
use log::info;
use redirect::CurlRedirector;
use regex::Regex;
use std::io;

fn main() -> Result<()> {
    env_logger::builder()
        .format_target(false)
        .format_timestamp(None)
        .init();

    let matches = App::new("fixred")
        .version(env!("CARGO_PKG_VERSION"))
        .about(
            "fixred is a tool to fix outdated links in text files. fixred replaces all HTTP and HTTPS \
            URLs with their redirect ones. fixred ignores invalid URLs or broken links to avoid false \
            positives on extracting URLs in text files.\n\n\
            fixred follows redirects repeatedly and uses the last URL to replace. The behavior can be \
            changed by --shallow flag to resolve the first redirect only.\n\n\
            Filtering URLs to be fixed is supported. See descriptions of --extract and --ignore options.\n\n\
            To enable verbose output, set $RUST_LOG environment variable. Setting RUST_LOG=info outputs \
            which file is being processed. Setting RUST_LOG=debug outputs what fixred is doing.\n\n\
            Visit https://github.com/rhysd/fixred#usage for more details with several examples.",
        )
        .global_setting(AppSettings::ColoredHelp)
        .arg(
            Arg::new("shallow")
                .short('s')
                .long("shallow")
                .about("Redirect only once when resolving a URL redirect")
        )
        .arg(
            Arg::new("extract")
                .short('e')
                .long("extract")
                .takes_value(true)
                .value_name("REGEX")
                .about("Fix URLs which are matched to this pattern"),
        )
        .arg(
            Arg::new("ignore")
                .short('r')
                .long("ignore")
                .takes_value(true)
                .value_name("REGEX")
                .about("Fix URLs which are NOT matched to this pattern"),
        )
        .arg(
            Arg::new("PATH")
                .about(
                    "Directory or file path to fix. When a directory path is given, all files in it \
                    are fixed recursively. When no path is given, fixred reads input from stdin and \
                    outputs the result to stdout",
                )
                .multiple_values(true),
        )
        .get_matches();

    let red = CurlRedirector::default()
        .extract(matches.value_of("extract").map(Regex::new).transpose()?)
        .ignore(matches.value_of("ignore").map(Regex::new).transpose()?)
        .shallow(matches.is_present("shallow"));

    if let Some(paths) = matches.values_of_os("PATH") {
        info!("Processing all files in given paths via command line arguments");
        let count = red.fix_all_files(paths)?;
        info!("Processed {} files", count);
    } else {
        info!("Fixing redirects in stdin");
        let stdin = io::stdin();
        let stdout = io::stdout();
        let count = red
            .fix(stdin.lock(), stdout.lock())
            .context("While processing stdin")?;
        info!("Fixed {} links in stdin", count);
    }

    Ok(())
}
