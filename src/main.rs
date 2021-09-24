use anyhow::Result;
use clap::{App, Arg};
use log::{debug, info};
use std::io;

mod redirect;
mod replace;
mod url;

fn main() -> Result<()> {
    env_logger::init();

    let matches = App::new("fixred")
        .arg(
            Arg::new("PATH")
                .about("Directory or file path to fix")
                .multiple_values(true),
        )
        .get_matches();

    let red = redirect::Redirector::default();
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
