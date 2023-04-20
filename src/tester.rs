use std::process::Command;

use crate::project_context::{BuildEnv, Context};
use anyhow::Result;

const TEST_ENV_VAR: &str = "CAPSULE_TEST_ENV";
const TESTS_DIR: &str = "tests";
pub struct Tester;

impl Tester {
    pub fn run(project_context: &Context, env: BuildEnv, test_name: Option<&str>) -> Result<()> {
        let env_arg = match env {
            BuildEnv::Debug => "debug",
            BuildEnv::Release => "release",
        };
        let workspace_dir = project_context.workspace_dir()?;
        let test_dir = workspace_dir.join(TESTS_DIR);
        // When workspace_dir is "contracts" we must mount build directory to /code/build so that test Loader can load the binary.
        let mut cmd = Command::new("cargo");
        cmd.arg("test")
            .current_dir(&test_dir)
            .env(TEST_ENV_VAR, env_arg);
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
