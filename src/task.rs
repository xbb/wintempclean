use std::env::current_exe;
use std::fmt::Write as FmtWrite;
use std::io::Write as IoWrite;

use std::process::{Command, Stdio};

use crate::output::open_log_file;
use anyhow::Result;

use super::*;

pub fn install_task(config: &Config) -> Result<()> {
    let task_name = clap::crate_name!();
    let args = parse_args(config)?;

    run_script(task_name, task_name, &args)?;

    Ok(())
}

fn parse_args(config: &Config) -> Result<Vec<String>> {
    let mut args: Vec<String> = vec![];

    if config.dry_run {
        args.push(String::from("--dry-run"));
    }

    if config.quiet {
        args.push(String::from("--quiet"));
    }

    if config.verbose {
        args.push(String::from("--verbose"));
    }

    if let Some(since) = config.since {
        args.push(String::from("--created-before"));
        args.push(format!("`\"{}`\"", humantime::format_duration(since)));
    }

    if let Some(log_path) = &config.log_path {
        test_log(log_path)
            .with_context(|| format!("Unable to create or open the log file {}", log_path))?;

        let log_path = format!("`\"{}`\"", log_path);

        args.push(String::from("--log"));
        args.push(log_path);
    }

    Ok(args)
}

fn test_log(log_path: &str) -> Result<()> {
    let log_path = Path::new(log_path);
    let existed = log_path.exists();
    open_log_file(log_path)?;

    if !existed {
        fs::remove_file(log_path)?;
    }

    Ok(())
}

fn run_script(task_path: &str, task_name: &str, clean_args: &[String]) -> Result<()> {
    let mut script = String::new();

    writeln!(
        script,
        "\
$ErrorActionPreference = \"Stop\"
$currentExe = \"{}\"
$action = New-ScheduledTaskAction -Execute \"$currentExe\" -Argument \"{}\"
$trigger = New-ScheduledTaskTrigger -AtStartup
$settings = New-ScheduledTaskSettingsSet
$task = New-ScheduledTask -Action $action -Trigger $trigger -Settings $settings
Register-ScheduledTask -Force -TaskPath \"{}\" -TaskName \"{}\" -InputObject $task -User SYSTEM
    ",
        current_exe()?.display(),
        clean_args.join(" "),
        task_path,
        task_name
    )?;

    // println!("Script:\n{}", script);

    let mut cmd = Command::new("powershell.exe");
    cmd.args(&["-NoProfile", "-WindowStyle", "Hidden", "-Command", "-"]);
    cmd.stdin(Stdio::piped());

    let mut process = cmd.spawn()?;
    let stdin = process.stdin.as_mut().unwrap();

    stdin.write_all(script.as_bytes())?;

    process.wait()?;

    Ok(())
}
