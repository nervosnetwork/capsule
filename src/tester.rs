use crate::project_context::{BuildEnv, Context};
use crate::recipe::rust::DOCKER_IMAGE;
use crate::signal::Signal;
use crate::util::docker::DockerCommand;
use anyhow::Result;
use std::collections::HashMap;

const TEST_ENV_VAR: &str = "CAPSULE_TEST_ENV";
pub struct Tester;

impl Tester {
    pub fn run(project_context: &Context, env: BuildEnv, signal: &Signal) -> Result<()> {
        let env_arg = match env {
            BuildEnv::Debug => "debug",
            BuildEnv::Release => "release",
        };
        let workspace_dir = project_context
            .workspace_dir()?
            .to_str()
            .expect("project path")
            .to_string();
        // When workspace_dir is "contracts" we must mount build directory to /code/build so that test Loader can load the binary.
        let build_dir = project_context
            .contracts_build_dir()
            .to_str()
            .expect("build dir")
            .to_string();
        let cmd = DockerCommand::with_context(
            project_context,
            DOCKER_IMAGE.to_string(),
            workspace_dir,
            &HashMap::new(),
        )
        .map_volume(build_dir, "/code/build".to_string())
        .fix_dir_permission("target".to_string())
        .fix_dir_permission("Cargo.lock".to_string());
        cmd.run(
            format!(
                "{}={} cargo test -p tests -- --nocapture",
                TEST_ENV_VAR, env_arg
            ),
            signal,
        )?;
        Ok(())
    }
}
