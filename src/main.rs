mod redirect;
mod replace;
mod resolve;
mod url;

use anyhow::Result;
use clap::{App, AppSettings, Arg};
use log::{debug, info};
use redirect::Redirector;
use regex::Regex;
use resolve::CurlResolver;
use std::io;

fn main() -> Result<()> {
    env_logger::init();

    let matches = App::new("fixred")
        .global_setting(AppSettings::ColoredHelp)
        .arg(
            Arg::new("select")
                .short('s')
                .long("select")
                .takes_value(true)
                .value_name("REGEX")
                .about("Redirect URLs which are matched to this pattern"),
        )
        .arg(
            Arg::new("reject")
                .short('r')
                .long("reject")
                .takes_value(true)
                .value_name("REGEX")
                .about("Redirect URLs which are NOT matched to this pattern"),
        )
        .arg(
            Arg::new("PATH")
                .about("Directory or file path to fix")
                .multiple_values(true),
        )
        .get_matches();

    let red = Redirector::default()
        .select(matches.value_of("select").map(Regex::new).transpose()?)
        .reject(matches.value_of("reject").map(Regex::new).transpose()?);

    if let Some(paths) = matches.values_of_os("PATH") {
        debug!("Some paths are given via arguments");
        red.fix_all_files::<_, CurlResolver>(paths)
    } else {
        info!("Fixing redirects in stdin");
        let stdin = io::stdin();
        let stdout = io::stdout();
        let count = red.fix::<_, _, CurlResolver>(stdin.lock(), stdout.lock())?;
        info!("Fixed {} links in stdin", count);
        Ok(())
    }
}
