use anyhow::{Context as ErrorContext, Result};
use include_dir::{include_dir, Dir, DirEntry};
use lazy_static::lazy_static;
use serde::Serialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
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

fn main() {
    let mut path = PathBuf::new();
    let mut name = env::args().skip(1).next().expect("name");
    if let Some(index) = name.rfind("/") {
        path.push(&name[..index]);
        name = name[index + 1..].to_string();
    } else {
        path.push(env::current_dir().expect("dir"));
    }
    new_project(name.to_string(), path).expect("new project");
}

// create a new project
fn new_project<P: AsRef<Path>>(name: String, path: P) -> Result<()> {
    let mut project_path: PathBuf = PathBuf::new();
    project_path.push(path);
    project_path.push(&name);
    // generate layouts
    println!("New project {:?}", &name);
    fs::create_dir(&project_path)
        .with_context(|| format!("directory exists {:?}", &project_path))?;
    for f in &["contracts", "build"] {
        let mut dir_path = PathBuf::new();
        dir_path.push(&project_path);
        dir_path.push(f);
        fs::create_dir(&dir_path)?;
    }
    println!("Created {:?}", &project_path);
    let mut default_contract_path = project_path.clone();
    default_contract_path.push("contracts");
    default_contract_path.push(&name);
    // generate cargo project
    Command::new("cargo")
        .arg("new")
        .arg(&default_contract_path)
        .spawn()?
        .wait()?;
    // initialize contract code
    let context = Context::from_serialize(&CreateProject {
        name: name.clone(),
        path: project_path.clone(),
    })?;
    for f in &["src/main.rs", "Cargo.toml"] {
        let template_path = format!("contract/{}", f);
        let content = TEMPLATES.render(&template_path, &context)?;
        let mut file_path = default_contract_path.clone();
        file_path.push(f);
        fs::write(file_path, content)?;
    }
    println!("Created contract {:?}", name);
    // generate files
    for f in &["Makefile", "README.md", "rust-toolchain"] {
        let content = TEMPLATES.render(f, &context)?;
        let mut file_path = project_path.clone();
        file_path.push(f);
        fs::write(file_path, content)?;
        println!("Created file {:?}", f);
    }
    println!("Done");
    Ok(())
}
