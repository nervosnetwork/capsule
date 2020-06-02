use anyhow::Result;
use std::io;

pub fn ask_for_confirm(msg: &str) -> Result<bool> {
    println!("{} (Yes/No)", msg);
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(["y", "yes"].contains(&buf.trim().to_lowercase().as_str()))
}
