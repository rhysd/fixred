use anyhow::Result;
use clap::{App, AppSettings, Arg};
use log::{debug, info};
use regex::Regex;
use std::io;

mod redirect;
mod replace;
mod url;

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

    let mut red = redirect::Redirector::default();
    if let Some(r) = matches.value_of("select").map(Regex::new).transpose()? {
        debug!("Regex to select URLs: {:?}", r);
        red = red.select(r);
    }
    if let Some(r) = matches.value_of("reject").map(Regex::new).transpose()? {
        debug!("Regex to reject URLs: {:?}", r);
        red = red.reject(r);
    }
    let red = red;

    if let Some(paths) = matches.values_of_os("PATH") {
        debug!("Some paths are given via arguments");
        red.fix_all_files(paths)
    } else {
        info!("Fixing redirects in stdin");
        let stdin = io::stdin();
        let stdout = io::stdout();
        let count = red.fix(stdin.lock(), stdout.lock())?;
        info!("Fixed {} links in stdin", count);
        Ok(())
    }
}
