use anyhow::Result;
use std::io;
use std::process::Command;

pub fn ask_for_confirm(msg: &str) -> Result<bool> {
    println!("{} (Yes/No)", msg);
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(["y", "yes"].contains(&buf.trim().to_lowercase().as_str()))
}

pub fn build_docker_cmd(shell_cmd: &str, code_path: &str, docker_image: &str) -> Result<Command> {
    let mut cmd = Command::new("docker");
    let uid = users::get_current_uid();
    let gid = users::get_current_gid();
    let user = users::get_current_username()
        .expect("user")
        .to_str()
        .expect("username")
        .to_string();
    cmd.args(&[
        "run",
        format!("-eUID={}", uid).as_str(),
        format!("-eGID={}", gid).as_str(),
        format!("-eUSER={}", user).as_str(),
        "--rm",
        "-v",
        format!("{}:/code", code_path).as_str(),
        docker_image,
        "bash",
        "-c",
        format!("{}", shell_cmd).as_str(),
    ]);
    Ok(cmd)
}
