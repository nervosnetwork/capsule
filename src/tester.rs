use std::process::Command;

use crate::project_context::{BuildEnv, Context};
use crate::recipe::rust::DOCKER_IMAGE;
use crate::signal::Signal;
use crate::util::docker::DockerCommand;
use anyhow::Result;

const TESTS_DIR: &str = "tests";
pub struct Tester;

impl Tester {
    pub fn run(project_context: &Context, env: BuildEnv, test_name: Option<&str>) -> Result<()> {
        let workspace_dir = project_context.workspace_dir()?;
        let test_dir = workspace_dir.join(TESTS_DIR);
        // When workspace_dir is "contracts" we must mount build directory to /code/build so that test Loader can load the binary.
        let mut cmd = Command::new("cargo");
        cmd.arg("test").current_dir(&test_dir);
        if env == BuildEnv::Release {
            cmd.arg("--release");
        }
        if let Some(test_name) = test_name {
            cmd.arg(test_name);
        }
        cmd.arg("--").arg("--nocapture");
        let status = cmd.status()?;
        if !status.success() {
            return Err(anyhow::anyhow!(
                "cargo test failed: {}",
                status.code().unwrap_or(-1)
            ));
        }
        Ok(())
    }
}
