mod app;
mod config;
mod output;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use humantime::format_duration;

use crate::app::build_app;
use crate::config::{build_config, Config};
use crate::output::{init_logger, print_err};

#[macro_use]
extern crate log;

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
    if let Err(err) = try_main() {
        if log_enabled!(log::Level::Error) {
            print_err(err);
        } else {
            eprintln!("{:?}", err);
        }
        std::process::exit(1);
    }
}

fn try_main() -> Result<()> {
    let matches = build_app().get_matches();
    let config = build_config(matches)?;

    init_logger(&config)?;

    if let Some(duration) = config.since {
        info!(
            "Removing temporary files and directories older than {}",
            format_duration(duration)
        );
    } else {
        info!("Removing all temporary files and directories");
    }

    begin_cleaning(&config)
}

fn begin_cleaning(config: &Config) -> Result<()> {
    for tmp_path in get_temp_directories()? {
        if tmp_path.exists() {
            debug!("Cleaning: {:?}", &tmp_path);

            if let Ok(stats) = remove_dir_contents(&tmp_path, config, false) {
                info!(
                    "Removed {} entries ({}) with {} errors from path {}",
                    stats.removed_count,
                    format_bytes(stats.removed_bytes as f64),
                    stats.errors_total,
                    tmp_path.display()
                );
            }
        }
    }

    Ok(())
}

fn get_temp_directories() -> Result<Vec<PathBuf>> {
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

        // Don't mind create date if subdir or no duration given
        if skip_date_check
            || config.since.is_none()
            || create_date_older_than_duration(&meta, config.since.unwrap())
        {
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
            if let Err(err) = remove_entry(&entry, config) {
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

    debug!("Removing{} {}", dry_run_tag, path.display());

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

fn create_date_older_than_duration(meta: &fs::Metadata, duration: Duration) -> bool {
    let elapsed = (|| -> Result<Duration> { Ok(meta.created()?.elapsed()?) })();

    match elapsed {
        Ok(elapsed) => elapsed >= duration,
        Err(err) => {
            // Warn and return false
            print_err(err);
            false
        }
    }
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
