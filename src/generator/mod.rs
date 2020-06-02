use crate::recipe::rust::DOCKER_IMAGE;
use crate::signal::Signal;
use crate::util::DockerCommand;
use anyhow::{Context as ErrorContext, Result};
use include_dir::{include_dir, Dir, DirEntry};
use lazy_static::lazy_static;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use tera::{self, Context, Tera};

const TEMPLATES_DIR: Dir = include_dir!("templates/rust");

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = Tera::default();
        for entry in TEMPLATES_DIR.find("**/*").expect("find templates") {
            let f = match entry {
                DirEntry::File(f) => f,
                _ => continue,
            };
            let path = f.path().to_str().expect("template path");
            let contents = String::from_utf8(f.contents().to_vec()).expect("template contents");
            tera.add_raw_template(path, &contents)
                .expect("failed to add template");
        }
        tera
    };
}

#[derive(Serialize)]
struct CreateProject {
    name: String,
    path: PathBuf,
}

#[derive(Serialize)]
struct CreateContract {
    name: String,
}

fn new_contract<P: AsRef<Path>>(name: String, path: P, signal: &Signal) -> Result<()> {
    let context = Context::from_serialize(&CreateContract { name: name.clone() })?;
    // generate contract
    let path = path.as_ref().to_str().expect("path");
    let cmd = DockerCommand::with_config(DOCKER_IMAGE.to_string(), path.to_string(), None)
        .fix_dir_permission(name.clone());
    cmd.run(format!("cargo new {}", name), signal)?;
    let mut contract_path = PathBuf::new();
    contract_path.push(path);
    contract_path.push(name);
    // initialize contract code
    for f in &["src/main.rs", "Cargo.toml"] {
        let template_path = format!("contract/{}", f);
        let content = TEMPLATES.render(&template_path, &context)?;
        let mut file_path = contract_path.clone();
        file_path.push(f);
        fs::write(file_path, content)?;
    }
    Ok(())
}

fn gen_project_layout<P: AsRef<Path>>(name: String, project_path: P) -> Result<()> {
    let project_path = {
        let mut path = PathBuf::new();
        path.push(project_path);
        path
    };
    fs::create_dir(&project_path)
        .with_context(|| format!("directory exists {:?}", &project_path))?;
    for f in &[
        "contracts",
        "build",
        "migrations",
        ".cache",
        ".cache/.cargo",
    ] {
        let mut dir_path = PathBuf::new();
        dir_path.push(&project_path);
        dir_path.push(f);
        fs::create_dir(&dir_path)?;
        dir_path.push(".gitkeep");
        fs::File::create(&dir_path)?;
    }
    // generate files
    let context = Context::from_serialize(&CreateProject {
        name: name.clone(),
        path: project_path.clone(),
    })?;
    for f in &[
        "capsule.toml",
        "deployment.toml",
        "README.md",
        "Cargo.toml",
        ".gitignore",
    ] {
        let content = TEMPLATES.render(f, &context)?;
        let mut file_path = project_path.clone();
        file_path.push(f);
        fs::write(file_path, content)?;
        println!("Created file {:?}", f);
    }
    Ok(())
}

fn gen_project_test<P: AsRef<Path>>(name: String, project_path: P, signal: &Signal) -> Result<()> {
    const DEFAULT_TESTS_DIR: &str = "tests";

    let project_path = project_path.as_ref().to_str().expect("path");
    let cmd = DockerCommand::with_config(DOCKER_IMAGE.to_string(), project_path.to_string(), None)
        .fix_dir_permission(DEFAULT_TESTS_DIR.to_string());
    cmd.run(format!("cargo new {} --lib", DEFAULT_TESTS_DIR), signal)?;
    let project_path = {
        let mut path = PathBuf::new();
        path.push(project_path);
        path
    };
    // initialize tests code
    let context = Context::from_serialize(&CreateProject {
        name: name.clone(),
        path: project_path.clone(),
    })?;
    let mut tests_path = project_path;
    tests_path.push(DEFAULT_TESTS_DIR);
    for f in &["src/lib.rs", "src/tests.rs", "Cargo.toml"] {
        let template_path = format!("tests/{}", f);
        let content = TEMPLATES.render(&template_path, &context)?;
        let mut file_path = tests_path.clone();
        file_path.push(f);
        fs::write(file_path, content)?;
    }
    Ok(())
}

// create a new project
pub fn new_project<P: AsRef<Path>>(name: String, path: P, signal: &Signal) -> Result<()> {
    let mut project_path: PathBuf = PathBuf::new();
    project_path.push(path);
    project_path.push(&name);
    // generate layouts
    println!("New project {:?}", &name);
    gen_project_layout(name.clone(), &project_path)?;
    println!("Created {:?}", &project_path);
    // generate contract
    let mut contracts_path = project_path.clone();
    contracts_path.push("contracts");
    new_contract(name.clone(), &contracts_path, signal)?;
    println!("Created contract {:?}", name);
    // generate contract tests
    println!("Created tests");
    gen_project_test(name, &project_path, signal)?;
    println!("Done");
    Ok(())
}
