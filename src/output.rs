use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use simplelog::{CombinedLogger, LevelFilter, SimpleLogger, WriteLogger};

use crate::Config;

pub fn print_err(err: anyhow::Error) {
    error!("Error: {}", err);
    err.chain()
        .skip(1)
        .for_each(|cause| error!("  Cause: {}", cause));
    error!("");
}

pub fn init_logger(config: &Config) -> Result<()> {
    let filter = if config.verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    let mut log_config = simplelog::ConfigBuilder::default();
    log_config.set_max_level(LevelFilter::Off);
    log_config.set_target_level(LevelFilter::Off);
    log_config.set_thread_level(LevelFilter::Off);
    log_config.set_time_to_local(true);

    let mut loggers: Vec<Box<(dyn simplelog::SharedLogger + 'static)>> = vec![];

    if !config.quiet || config.install_task {
        loggers.push(SimpleLogger::new(filter, log_config.build()));
    }

    if !config.install_task {
        if let Some(log_path) = &config.log_path {
            // Open or create file for writing (append)
            let log_file = open_log_file(Path::new(log_path))?;

            loggers.push(WriteLogger::new(filter, log_config.build(), log_file));
        }
    }

    Ok(CombinedLogger::init(loggers)?)
}

pub fn open_log_file(log_path: &Path) -> Result<fs::File> {
    // If the path exists it may be a directory
    if log_path.exists() && !log_path.is_file() {
        bail!("Invalid path specified for log file");
    }

    fs::File::options()
        .append(true)
        .create(true)
        .open(&log_path)
        .with_context(|| {
            format!(
                "Unable to create or open the log file {}",
                log_path.display()
            )
        })
}
