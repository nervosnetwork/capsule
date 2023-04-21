extern crate includedir_codegen;

use includedir_codegen::Compression;
use std::io::ErrorKind;

fn main() {
    // include templates
    includedir_codegen::start("FILES")
        .dir("templates", Compression::Gzip)
        .build("templates.rs")
        .unwrap();

    let get_command_id = std::process::Command::new("git")
        .args([
            "describe",
            "--dirty",
            "--always",
            "--match",
            "__EXCLUDE__",
            "--abbrev=7",
        ])
        .output();
    let commit_id = match get_command_id {
        Ok(output) => String::from_utf8(output.stdout).expect("commit id"),
        Err(err) => {
            if let ErrorKind::NotFound = err.kind() {
                panic!("error when get commit id: `git` was not found!");
            } else {
                panic!("error when get commit id: {}", err);
            }
        }
    };

    println!("cargo:rustc-env=COMMIT_ID={}", commit_id);
}
