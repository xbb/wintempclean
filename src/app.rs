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
                .help("Removes only the files created before the specified duration (60s, 10d, 1m, 1y)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .short('n')
                .help("Doesn't actually remove the files")
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
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .help("Shows what files are removed")
        )
}
