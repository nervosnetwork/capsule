use anyhow::Result;
use std::process::Command;

pub fn setup() -> Result<()> {
    Command::new("bash")
        .arg("-c")
        .arg(
            "which ckb-binary-patcher || \
            cargo install --force --git https://github.com/xxuejie/ckb-binary-patcher.git
            ",
        )
        .spawn()?
        .wait()?;
    Ok(())
}
