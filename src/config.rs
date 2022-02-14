use std::time::Duration;

use anyhow::Result;
use clap::ArgMatches;

pub struct Config {
    pub dry_run: bool,
    pub install_task: bool,
    pub log_path: Option<String>,
    pub quiet: bool,
    pub since: Option<Duration>,
    pub verbose: bool,
}

pub fn build_config(matches: &ArgMatches) -> Result<Config> {
    let since = match matches.value_of("created-before") {
        Some(value) => Some(humantime::parse_duration(value)?),
        _ => None,
    };

    let config = Config {
        dry_run: matches.is_present("dry-run"),
        install_task: matches.is_present("install-task"),
        quiet: matches.is_present("quiet"),
        verbose: matches.is_present("verbose"),
        log_path: matches.value_of("log").map(|x| x.to_string()),
        since,
    };

    Ok(config)
}
