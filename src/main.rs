mod generator;

use generator::new_project;
use std::env;
use std::path::PathBuf;
use std::process::{exit, Command};

fn main() {
    let mut args = env::args().skip(1);
    let command = args.next().expect("command");
    match &command[..] {
        "new" => {
            let mut name = args.next().expect("name");
            let mut path = PathBuf::new();
            if let Some(index) = name.rfind("/") {
                path.push(&name[..index]);
                name = name[index + 1..].to_string();
            } else {
                path.push(env::current_dir().expect("dir"));
            }
            new_project(name.to_string(), path).expect("new project");
        }
        "test" => {
            let exit_code = Command::new("cargo")
                .arg("test")
                .spawn()
                .expect("spawn")
                .wait()
                .expect("wait command");
            exit(exit_code.code().unwrap_or(1));
        }
        _ => {
            println!("unrecognize command '{}'", command);
            exit(1);
        }
    }
}
