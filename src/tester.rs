use crate::project_context::{BuildEnv, Context};
use anyhow::Result;
use xshell::{cmd, Shell};

const TEST_ENV_VAR: &str = "CAPSULE_TEST_ENV";
const TESTS_DIR: &str = "tests";
pub struct Tester;

impl Tester {
    pub fn run(project_context: &Context, env: BuildEnv, test_name: Option<&str>) -> Result<()> {
        let env_arg = match env {
            BuildEnv::Debug => "debug",
            BuildEnv::Release => "release",
        };
        println!("{TEST_ENV_VAR}={env_arg}");
        let workspace_dir = project_context.workspace_dir()?;
        let test_dir = workspace_dir.join(TESTS_DIR);

        let sh = Shell::new()?;
        sh.change_dir(test_dir);
        cmd!(sh, "cargo test {test_name...} -- --nocapture")
            .env(TEST_ENV_VAR, env_arg)
            .run()?;
        Ok(())
    }
}
