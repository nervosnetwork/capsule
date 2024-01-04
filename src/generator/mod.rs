use crate::util::git;
use crate::version::Version;
use anyhow::{bail, Context as ErrorContext, Result};
use lazy_static::lazy_static;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use tera::{self, Context, Tera};

extern crate includedir;
extern crate phf;

include!(concat!(env!("OUT_DIR"), "/templates.rs"));

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = Tera::default();
        for path in FILES.file_names() {
            let filename = path.strip_prefix("templates/").expect("remove prefix");
            let content = {
                let c = FILES.get(path).expect("read template");
                String::from_utf8(c.to_vec()).expect("template contents")
            };
            tera.add_raw_template(filename, &content)
                .expect("failed to add template");
        }
        tera
    };
}

#[derive(Serialize)]
struct CreateProject {
    name: String,
    path: PathBuf,
    version: String,
}

#[derive(Serialize)]
pub struct CreateContract {
    pub name: String,
}

fn gen_project_layout<P: AsRef<Path>>(name: String, project_path: P) -> Result<()> {
    let project_path = {
        let mut path = PathBuf::new();
        path.push(project_path);
        path
    };
    fs::create_dir(&project_path)
        .with_context(|| format!("directory exists {:?}", &project_path))?;
    for f in &["contracts", "build"] {
        let mut dir_path = PathBuf::new();
        dir_path.push(&project_path);
        dir_path.push(f);
        fs::create_dir(&dir_path)?;
        dir_path.push(".gitkeep");
        fs::File::create(&dir_path)?;
    }
    // generate files
    let context = Context::from_serialize(&CreateProject {
        name,
        path: project_path.clone(),
        version: Version::current().to_string(),
    })?;
    for (f, template_name) in &[
        ("capsule.toml", None),
        ("README.md", None),
        ("rust-toolchain.toml", None),
        ("Cross.toml", None),
        ("Cargo.toml", Some("Cargo-manifest.toml")),
        (".gitignore", None),
    ] {
        let content = TEMPLATES.render(template_name.unwrap_or(f), &context)?;
        let mut file_path = project_path.clone();
        file_path.push(f);
        fs::write(file_path, content)?;
        println!("Created file {:?}", f);
    }
    git::init(&project_path)?;
    Ok(())
}

fn gen_project_test<P: AsRef<Path>>(name: String, project_path: P) -> Result<()> {
    const DEFAULT_TESTS_DIR: &str = "tests";

    let project_path = project_path.as_ref().to_str().expect("path");
    let output = std::process::Command::new("cargo")
        .args(["new", DEFAULT_TESTS_DIR, "--lib", "--vcs", "none"])
        .current_dir(project_path)
        .output()?;
    if !output.status.success() {
        bail!("failed to generate tests, status: {}", output.status);
    }
    let project_path = {
        let mut path = PathBuf::new();
        path.push(project_path);
        path
    };
    // initialize tests code
    let context = Context::from_serialize(&CreateProject {
        name,
        path: project_path.clone(),
        version: Version::current().to_string(),
    })?;
    let mut tests_path = project_path;
    tests_path.push(DEFAULT_TESTS_DIR);
    for (f, template_name) in &[
        ("src/lib.rs", None),
        ("src/tests.rs", None),
        ("Cargo.toml", Some("Cargo-manifest.toml")),
    ] {
        log::trace!("[modify test file] {}", f);
        let template_path = format!("rust/tests/{}", template_name.unwrap_or(f));
        let content = TEMPLATES.render(&template_path, &context)?;
        let mut file_path = tests_path.clone();
        file_path.push(f);
        fs::write(file_path, content)?;
    }
    Ok(())
}

// create a new project
pub fn new_project<P: AsRef<Path>>(name: String, path: P) -> Result<PathBuf> {
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
    // generate contract tests
    println!("Created tests");
    gen_project_test(name, &project_path)?;
    Ok(project_path)
}
