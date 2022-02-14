use clap::{App, Arg};

pub fn build_app() -> App<'static> {
    App::new(clap::crate_name!())
        .version(clap::crate_version!())
        .arg(
            Arg::new("created-before")
                .long("created-before")
                .short('b')
                .takes_value(true)
                .value_name("duration")
                .number_of_values(1)
                .help("Removes only the files created before the specified duration (60s, 10m, 10h, 10d, 10days 2min, etc...)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .short('n')
                .help("Doesn't actually remove the files")
        )
        .arg(
            Arg::new("install-task")
            .long("install-task")
            .help("Creates a new task in the scheduler for cleaning during startup as SYSTEM user")
        )
        .arg(
            Arg::new("log")
                .long("log")
                .short('l')
                .takes_value(true)
                .value_name("log file")
                .number_of_values(1)
                .help("Log output to a file")
        )
        .arg(
            Arg::new("quiet")
                .long("quiet")
                .short('q')
                .help("Suppress all terminal output")
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .help("Shows what files are removed")
        )
}
