use std::fmt::Write as FmtWrite;
use std::io::Write as IoWrite;
use std::os::windows::process::CommandExt;
use std::process::Stdio;
use std::{io, thread};

use anyhow::{bail, Result};

use crate::output::open_log_file;
use crate::windows::is_app_elevated;

use super::*;

pub fn install_task(config: &Config) -> Result<()> {
    if !is_app_elevated() {
        bail!("--install-task required administrator privileges");
    }

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
trap
{{
    write-output $_
    exit 1
}}
$ErrorActionPreference = \"Stop\"
$currentExe = \"{}\"
$action = New-ScheduledTaskAction -Execute \"$currentExe\" -Argument \"{}\"
$trigger = New-ScheduledTaskTrigger -AtStartup
$settings = New-ScheduledTaskSettingsSet
$task = New-ScheduledTask -Action $action -Trigger $trigger -Settings $settings
Register-ScheduledTask -Force -TaskPath \"{}\" -TaskName \"{}\" -InputObject $task -User SYSTEM
    ",
        std::env::current_exe()?.display(),
        clean_args.join(" "),
        task_path,
        task_name
    )?;

    let mut process = std::process::Command::new("powershell.exe")
        // -WindowStyle Hidden not included because it makes the child process detach early
        .args(&["-NonInteractive", "-NoProfile", "-Command", "-"])
        // Don't create a window for the spawned process
        .creation_flags(winapi::um::winbase::CREATE_NO_WINDOW)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn()?;

    let mut child_out = process.stdout.take().unwrap();
    let mut child_err = process.stderr.take().unwrap();
    let mut child_in = process.stdin.take().unwrap();

    let out_thread = thread::spawn(move || {
        io::copy(&mut child_out, &mut io::stdout()).unwrap();
    });

    let err_thread = thread::spawn(move || {
        io::copy(&mut child_err, &mut io::stderr()).unwrap();
    });

    let in_thread = thread::spawn(move || {
        child_in.write_all(script.as_bytes()).unwrap();
    });

    let status = process.wait()?;
    in_thread.join().unwrap();
    err_thread.join().unwrap();
    out_thread.join().unwrap();

    if status.success() {
        println!("Task created successfully");
    } else {
        bail!("Error while creating the task");
    }

    Ok(())
}
