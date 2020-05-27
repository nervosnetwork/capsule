use crate::project_context::{BuildEnv, Context};
use crate::recipe::rust::DOCKER_IMAGE;
use crate::signal::Signal;
use crate::util::DockerCommand;
use anyhow::Result;

const TEST_ENV_VAR: &str = "CAPSULE_TEST_ENV";
pub struct Tester;

impl Tester {
    pub fn run(project_context: &Context, env: BuildEnv, signal: &Signal) -> Result<()> {
        let env_arg = match env {
            BuildEnv::Debug => "debug",
            BuildEnv::Release => "release",
        };
        let project_path = project_context
            .project_path
            .to_str()
            .expect("project path")
            .to_string();
        let cmd =
            DockerCommand::with_context(project_context, DOCKER_IMAGE.to_string(), project_path)
                .fix_dir_permission("target".to_string());
        cmd.run(
            format!(
                "cd /code && {}={} cargo test -- --nocapture",
                TEST_ENV_VAR, env_arg
            ),
            signal,
        )?;
        Ok(())
    }
}
