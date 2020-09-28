use crate::project_context::Context;
use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::{Command, ExitStatus};

const GIT_BIN: &str = "git";

fn wait(status: ExitStatus) -> Result<()> {
    if status.success() {
        return Ok(());
    } else {
        let err = anyhow!("{} exit with code {:?}", GIT_BIN, status.code());
        return Err(err);
    }
}

pub fn init<P: AsRef<Path>>(dir: P) -> Result<()> {
    let status = Command::new(GIT_BIN)
        .arg("init")
        .current_dir(dir)
        .status()?;

    wait(status)
}

pub fn add_submodule(context: &Context, url: &str, rel_path: &str, commit_id: &str) -> Result<()> {
    let status = Command::new(GIT_BIN)
        .arg("submodule")
        .arg("add")
        .arg(url)
        .arg(rel_path)
        .current_dir(&context.project_path)
        .status()?;
    wait(status)?;
    let mut submodule_path = context.project_path.clone();
    submodule_path.push(rel_path);
    let status = Command::new(GIT_BIN)
        .arg("checkout")
        .arg("--quiet")
        .arg(commit_id)
        .current_dir(submodule_path)
        .status()?;
    wait(status)
}
