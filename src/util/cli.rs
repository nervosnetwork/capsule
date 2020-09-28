use crate::signal::Signal;
use anyhow::{anyhow, Result};
use log::debug;
use std::io;
use std::path::Path;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

pub fn ask_for_confirm(msg: &str) -> Result<bool> {
    println!("{} (Yes/No)", msg);
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(["y", "yes"].contains(&buf.trim().to_lowercase().as_str()))
}

pub fn run<P: AsRef<Path>>(shell_cmd: String, workdir: P, signal: &Signal) -> Result<()> {
    debug!("Run command: {}", shell_cmd);
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(&shell_cmd).current_dir(workdir);
    let mut child = cmd.spawn()?;
    while signal.is_running() {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    return Ok(());
                } else {
                    let err = anyhow!("process exit with code {:?}", status.code());
                    return Err(err);
                }
            }
            Ok(None) => {
                sleep(Duration::from_millis(300));
                continue;
            }
            Err(e) => panic!("error attempting to wait: {}", e),
        }
    }
    println!("Exiting...");
    child.kill()?;
    signal.exit()
}
