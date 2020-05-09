use crate::recipe::rust::DOCKER_IMAGE;
use crate::util::build_docker_cmd;
use anyhow::Result;
use std::path::Path;
use std::process::ExitStatus;

pub struct Tester;

impl Tester {
    pub fn run<P: AsRef<Path>>(project_path: P) -> Result<ExitStatus> {
        Ok(build_docker_cmd(
            "cd /code && cargo test;\
         EXITCODE=$?;chown -R $UID:$GID target; exit $EXITCODE",
            project_path.as_ref().to_str().expect("path"),
            DOCKER_IMAGE,
        )?
        .spawn()?
        .wait()?)
    }
}
