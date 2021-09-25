mod redirect;
mod replace;
mod resolve;
mod url;

use anyhow::Result;
use clap::{App, AppSettings, Arg};
use log::info;
use redirect::Redirector;
use regex::Regex;
use resolve::CurlResolver;
use std::io;

fn main() -> Result<()> {
    env_logger::init();

    let matches = App::new("fixred")
        .version(env!("CARGO_PKG_VERSION"))
        .about(
            "fixred is a tool to fix outdated links in text files. fixred replaces all HTTP and HTTPS \
            URLs with their redirect ones. fixred follows redirects repeatedly and uses the last URL to \
            replace.\n\n\
            Filtering URLs to be fixed is supported. See descriptions of --select and --reject options.\n\n\
            To enable verbose output, set $RUST_LOG environment variable. Setting RUST_LOG=info outputs \
            which file is being processed. Setting RUST_LOG=debug outputs what fixred is doing.\n\n\
            Visit https://github.com/rhysd/fixred#readme for more details.",
        )
        .global_setting(AppSettings::ColoredHelp)
        .arg(
            Arg::new("select")
                .short('s')
                .long("select")
                .takes_value(true)
                .value_name("REGEX")
                .about("Fix URLs which are matched to this pattern."),
        )
        .arg(
            Arg::new("reject")
                .short('r')
                .long("reject")
                .takes_value(true)
                .value_name("REGEX")
                .about("Fix URLs which are NOT matched to this pattern."),
        )
        .arg(
            Arg::new("PATH")
                .about(
                    "Directory or file path to fix. When a directory path is given, all files in it \
                    are fixed recursively. When no path is given, fixred reads input from stdin and \
                    outputs the result to stdout.",
                )
                .multiple_values(true),
        )
        .get_matches();

    let red = Redirector::<CurlResolver>::default()
        .select(matches.value_of("select").map(Regex::new).transpose()?)
        .reject(matches.value_of("reject").map(Regex::new).transpose()?);

    if let Some(paths) = matches.values_of_os("PATH") {
        info!("Processing all files in given paths via command line arguments");
        let count = red.fix_all_files(paths)?;
        info!("Processed {} files", count);
    } else {
        info!("Fixing redirects in stdin");
        let stdin = io::stdin();
        let stdout = io::stdout();
        let count = red.fix(stdin.lock(), stdout.lock())?;
        info!("Fixed {} links in stdin", count);
    }

    Ok(())
}
