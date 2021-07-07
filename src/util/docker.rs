use crate::project_context::Context;
use crate::signal::Signal;
use anyhow::{anyhow, Result};
use log::debug;
use std::collections::HashMap;
use std::env;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

const DOCKER_BIN: &str = "docker";

struct Volume {
    volume: String,
    container: String,
}

struct Port {
    host: usize,
    container: usize,
}

pub struct DockerCommand {
    bin: String,
    uid: u32,
    gid: u32,
    user: String,
    docker_image: String,
    code_path: String,
    patch_cargo_cache: bool,
    fix_permission_files: Vec<String>,
    mapping_ports: Vec<Port>,
    mapping_volumes: Vec<Volume>,
    host_network: bool,
    name: Option<String>,
    daemon: bool,
    tty: bool,
    workdir: String,
    inherited_env: Vec<&'static str>,
    custom_env: HashMap<String, String>,
}

impl DockerCommand {
    pub fn with_context(
        _context: &Context,
        docker_image: String,
        code_path: String,
        custom_env: &HashMap<String, String>,
    ) -> Self {
        Self::with_config(docker_image, code_path, &custom_env)
    }

    pub fn with_config(
        docker_image: String,
        code_path: String,
        custom_env: &HashMap<String, String>,
    ) -> Self {
        let bin = DOCKER_BIN.to_string();
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
            patch_cargo_cache: true,
            fix_permission_files: Vec::new(),
            mapping_ports: Vec::new(),
            mapping_volumes: Vec::new(),
            host_network: false,
            name: None,
            daemon: false,
            tty: false,
            workdir: "/code".to_string(),
            inherited_env: vec!["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY"],
            custom_env: custom_env.clone(),
        }
    }

    pub fn host_network(mut self, enable: bool) -> Self {
        self.host_network = enable;
        self
    }

    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn daemon(mut self, daemon: bool) -> Self {
        self.daemon = daemon;
        self
    }

    pub fn tty(mut self, tty: bool) -> Self {
        self.tty = tty;
        self
    }

    pub fn workdir(mut self, dir: String) -> Self {
        self.workdir = dir;
        self
    }

    pub fn fix_dir_permission(mut self, dir: String) -> Self {
        self.fix_permission_files.push(dir);
        self
    }

    pub fn map_volume(mut self, volume: String, container: String) -> Self {
        self.mapping_volumes.push(Volume { volume, container });
        self
    }

    pub fn run(self, shell_cmd: String, signal: &Signal) -> Result<()> {
        debug!("Run command in docker: {}", shell_cmd);
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

    pub fn stop(name: &str) -> Result<()> {
        println!("Stop container {}...", name);
        let mut cmd = Command::new(DOCKER_BIN);
        cmd.args(&["stop", name]);
        let exit_status = cmd.spawn()?.wait()?;
        if !exit_status.success() {
            return Err(anyhow!(
                "failed to stop container {}, exit {}",
                name,
                exit_status.code().unwrap_or(0)
            ));
        }
        Ok(())
    }

    fn build(self, mut shell_cmd: String) -> Result<Command> {
        let DockerCommand {
            bin,
            uid,
            gid,
            user,
            docker_image,
            code_path,
            patch_cargo_cache,
            fix_permission_files,
            mapping_ports,
            mut mapping_volumes,
            host_network,
            name,
            daemon,
            tty,
            workdir,
            inherited_env,
            custom_env,
        } = self;

        let mut cmd = Command::new(bin);
        cmd.args(&[
            "run",
            format!("-eUID={}", uid).as_str(),
            format!("-eGID={}", gid).as_str(),
            format!("-eUSER={}", user).as_str(),
            "--rm",
            format!("-v{}:/code", code_path).as_str(),
            format!("-w{}", workdir).as_str(),
        ]);

        // reusing cargo cache
        // mapping local volume `capsule-cache` to reusing cargo cache
        if patch_cargo_cache {
            mapping_volumes.push(Volume {
                volume: "capsule-cache".to_string(),
                container: "/root/.cargo".to_string(),
            });
        }

        // mapping volumes
        for volumn in mapping_volumes {
            cmd.arg(format!("-v{}:{}", volumn.volume, volumn.container).as_str());
        }

        // mapping ports
        for port in mapping_ports {
            cmd.arg(format!("-p{}:{}", port.host, port.container).as_str());
        }

        // inject env
        for key in inherited_env {
            if let Ok(value) = env::var(key) {
                debug!("inherited env {}={}", key, value);
                cmd.arg(format!("-e{}:{}", key, value));
            }
        }

        // inject custom env
        for (key, value) in custom_env {
            debug!("custom env {}={}", key, value);
            cmd.arg(format!("-e{}:{}", key, value));
        }

        if host_network {
            cmd.arg("--network").arg("host");
        }

        if let Some(name) = name {
            cmd.arg("--name").arg(name);
        }

        if daemon {
            cmd.arg("-d");
        }

        if tty {
            cmd.arg("-it");
        }

        // fix files permission
        shell_cmd.push_str("; EXITCODE=$?");
        for f in &fix_permission_files {
            shell_cmd.push_str(
                format!("; test -f {f} -o -d {f} && chown -R $UID:$GID {f}", f = f).as_str(),
            );
        }
        shell_cmd.push_str("; exit $EXITCODE");
        cmd.args(&[docker_image.as_ref(), "bash", "-c", shell_cmd.as_str()]);

        Ok(cmd)
    }
}
