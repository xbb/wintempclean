use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{fs, io};

use anyhow::{anyhow, Context, Result};
use clap::{App, Arg, ArgMatches};
use humantime::format_duration;

struct Config {
    dry_run: bool,
    verbose: bool,
    since: Duration,
}

struct Stats {
    errors_total: u64,
    removed_bytes: u64,
    removed_count: u64,
}

impl Stats {
    fn new() -> Stats {
        Stats {
            errors_total: 0,
            removed_bytes: 0,
            removed_count: 0,
        }
    }

    fn add(&mut self, stats: Stats) {
        self.errors_total += stats.errors_total;
        self.removed_bytes += stats.removed_bytes;
        self.removed_count += stats.removed_count;
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{:?}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<i32> {
    let matches = build_app().get_matches();
    let config = build_config(matches)?;

    if !config.since.is_zero() {
        println!(
            "Removing temporary files and directories older than {}",
            format_duration(config.since)
        );
    } else {
        println!("Removing all temporary files and directories");
    }

    match begin_cleaning(&config) {
        Ok(()) => Ok(0),
        Err(e) => Err(anyhow!(e)),
    }
}

fn build_app() -> App<'static> {
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
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .help("Shows what files are removed")
        )
}

fn build_config(matches: ArgMatches) -> Result<Config> {
    let dry_run = matches.is_present("dry-run");

    let since = match matches.value_of("created-before") {
        Some(value) => humantime::parse_duration(value)?,
        None => Duration::from_secs(0),
    };

    Ok(Config {
        dry_run,
        verbose: matches.is_present("verbose"),
        since,
    })
}

fn begin_cleaning(config: &Config) -> Result<()> {
    for tmp_path in get_temp_directories()? {
        if tmp_path.exists() {
            if config.verbose {
                println!("Cleaning: {:?}", &tmp_path)
            }
            if let Ok(stats) = remove_dir_contents(&tmp_path, &config, false) {
                println!(
                    "Removed {} entries ({} {}) with {} errors from path {}",
                    stats.removed_count,
                    format_bytes(stats.removed_bytes as f64),
                    stats.removed_bytes,
                    stats.errors_total,
                    tmp_path.display()
                );
            }
        }
    }

    Ok(())
}

fn get_temp_directories() -> Result<Vec<PathBuf>, io::Error> {
    let mut dirs = vec![
        PathBuf::from(r"C:\Windows\Temp"),
        PathBuf::from(r"C:\ProgramData\Temp"),
    ];

    let users_dirs = fs::read_dir(r"C:\Users")?
        .into_iter()
        .map(|x| x.map(|entry| entry.path().join("AppData\\Local\\Temp\\")))
        .collect::<Result<Vec<_>, _>>()?;

    dirs.extend(users_dirs);

    Ok(dirs)
}

fn remove_dir_contents(path: &Path, config: &Config, skip_date_check: bool) -> Result<Stats> {
    let entries =
        fs::read_dir(path).with_context(|| format!("can't read dir {}", path.display()))?;

    let mut stats = Stats::new();

    // Loop every entry
    for entry in entries {
        let entry = entry?;

        let meta = fs::metadata(entry.path())
            .with_context(|| format!("can't read metadata {}", entry.path().display()));

        // Read metadata or report error
        let meta = match meta {
            Ok(result) => result,
            Err(err) => {
                stats.errors_total += 1;
                print_err(err);
                continue;
            }
        };

        // Store size for later
        let size = meta.len();

        let elapsed_since_create = meta.created()?.elapsed().unwrap_or_else(|err| {
            // Warn and return a default duration
            eprintln!("{:?}", err);
            Duration::from_secs(0)
        });

        // Don't mind create date if subdir
        if skip_date_check || elapsed_since_create >= config.since {
            // Recurse into subdir and sum stats
            if meta.is_dir() {
                // Try remove sub contents
                match remove_dir_contents(&entry.path(), config, true) {
                    Ok(sub_stats) => {
                        // Sum stats
                        stats.add(sub_stats);
                    }
                    Err(err) => {
                        // Error: return early
                        stats.errors_total += 1;
                        print_err(err);
                        return Ok(stats);
                    }
                };
            }

            // Remove entry or report error
            if let Err(err) = remove_entry(&entry, &config) {
                stats.errors_total += 1;
                print_err(err);
            } else {
                stats.removed_bytes += size;
                stats.removed_count += 1;
            }
        }
    }

    Ok(stats)
}

fn remove_entry(entry: &fs::DirEntry, config: &Config) -> Result<()> {
    let dry_run_tag = if config.dry_run { " (dry run)" } else { "" };
    let path = entry.path();

    if config.verbose {
        println!("Removing{} {}", dry_run_tag, path.display());
    }

    if !config.dry_run {
        return if path.is_dir() {
            // Remove dir and return
            fs::remove_dir(&path)
                .with_context(|| format!("failed to remove directory {}", path.display()))
        } else {
            // Remove file and return
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove file {}", path.display()))
        };
    }

    Ok(())
}

// https://www.sqlservercentral.com/blogs/powershell-using-exponents-and-logs-to-format-byte-sizes
fn format_bytes(bytes: f64) -> String {
    let units = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    let negative_sign = if bytes.is_sign_negative() { "-" } else { "" };
    let bytes = bytes.abs();

    if bytes < 1_f64 {
        return format!("{}{} {}", negative_sign, bytes, "B");
    }

    let pow2 = (bytes.ln() / 2_f64.ln()).floor();
    let idx = (pow2 / 10.0).floor().min((units.len() - 1) as f64) as i32;
    let scaled = bytes / 2_f64.powi(idx * 10);
    let unit = units[idx as usize];

    format!("{}{:.2} {}", negative_sign, scaled, unit)
}

fn print_err(err: anyhow::Error) {
    eprintln!("Error: {}", err);
    err.chain()
        .skip(1)
        .for_each(|cause| eprintln!("  Cause: {}", cause));
    eprintln!();
}
