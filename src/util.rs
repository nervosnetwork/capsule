use crate::project_context::Context;
use crate::signal::Signal;
use anyhow::{anyhow, Result};
use std::io;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

pub fn ask_for_confirm(msg: &str) -> Result<bool> {
    println!("{} (Yes/No)", msg);
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(["y", "yes"].contains(&buf.trim().to_lowercase().as_str()))
}

pub struct DockerCommand {
    bin: String,
    uid: u32,
    gid: u32,
    user: String,
    docker_image: String,
    code_path: String,
    cargo_dir_path: Option<String>,
    fix_permission_dirs: Vec<String>,
}

impl DockerCommand {
    pub fn with_context(context: &Context, docker_image: String, code_path: String) -> Self {
        let cargo_dir_path = context
            .cargo_cache_path()
            .to_str()
            .expect("path")
            .to_string();
        Self::with_config(docker_image, code_path, Some(cargo_dir_path))
    }

    pub fn with_config(
        docker_image: String,
        code_path: String,
        cargo_dir_path: Option<String>,
    ) -> Self {
        let bin = "docker".to_string();
        let uid = users::get_current_uid();
        let gid = users::get_current_gid();
        let user = users::get_current_username()
            .expect("user")
            .to_str()
            .expect("username")
            .to_string();
        DockerCommand {
            bin,
            uid,
            gid,
            user,
            docker_image,
            code_path,
            cargo_dir_path,
            fix_permission_dirs: Vec::new(),
        }
    }

    pub fn fix_dir_permission(mut self, dir: String) -> Self {
        self.fix_permission_dirs.push(dir);
        self
    }

    pub fn run(self, shell_cmd: String, signal: &Signal) -> Result<()> {
        let mut cmd = self.build(shell_cmd)?;
        let mut child = cmd.spawn()?;
        while signal.is_running() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if status.success() {
                        return Ok(());
                    } else {
                        let err = anyhow!("docker container exit with code {:?}", status.code());
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

    fn build(self, mut shell_cmd: String) -> Result<Command> {
        let DockerCommand {
            bin,
            uid,
            gid,
            user,
            docker_image,
            code_path,
            cargo_dir_path,
            mut fix_permission_dirs,
        } = self;

        let mut cmd = Command::new(bin);
        cmd.args(&[
            "run",
            format!("-eUID={}", uid).as_str(),
            format!("-eGID={}", gid).as_str(),
            format!("-eUSER={}", user).as_str(),
            "--rm",
            format!("-v{}:/code", code_path).as_str(),
        ]);
        if let Some(cargo_dir_path) = cargo_dir_path {
            cmd.args(&[
                format!("-v{}/git:/root/.cargo/git", cargo_dir_path).as_str(),
                format!("-v{}/git:/root/.cargo/registry", cargo_dir_path).as_str(),
            ]);
            fix_permission_dirs.push("/root/.cargo".to_string());
        }
        // fix files permission
        shell_cmd.push_str("; EXITCODE=$?");
        for dir in &fix_permission_dirs {
            shell_cmd.push_str(format!("; chown -R $UID:$GID {}", dir).as_str());
        }
        shell_cmd.push_str("; exit $EXITCODE");
        cmd.args(&[docker_image.as_ref(), "bash", "-c", shell_cmd.as_str()]);

        Ok(cmd)
    }
}
