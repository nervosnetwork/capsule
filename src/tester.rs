use crate::project_context::Context;
use crate::recipe::rust::DOCKER_IMAGE;
use crate::util::DockerCommand;
use anyhow::Result;

pub struct Tester;

impl Tester {
    pub fn run(project_context: &Context) -> Result<()> {
        let project_path = project_context
            .project_path
            .to_str()
            .expect("project path")
            .to_string();
        let cmd =
            DockerCommand::with_context(project_context, DOCKER_IMAGE.to_string(), project_path)
                .fix_dir_permission("target".to_string());
        cmd.run("cd /code && cargo test".to_string())?;
        Ok(())
    }
}
