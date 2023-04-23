use anyhow::Error;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BIN_PATH: &str = "target/debug/capsule";

pub fn run() {
    println!("Testing capsule ...");
    let cur_dir = env::current_dir().expect("current dir");
    let bin_path = {
        let mut path = PathBuf::new();
        path.push(&cur_dir);
        path.push(BIN_PATH);
        path.to_str().expect("capsule bin path").to_string()
    };
    let tmp_dir = {
        let mut path = PathBuf::new();
        path.push(&cur_dir);
        path.push("tmp");
        path
    };
    fs::create_dir_all(&tmp_dir).expect("create dir");

    // test cases
    test_build(&tmp_dir, &bin_path, "rust-demo", "rust").expect("rust demo");
    test_build(&tmp_dir, &bin_path, "c-demo", "c").expect("c demo");
    test_build_sharedlib(&tmp_dir, &bin_path, "c-sharedlib-demo", "c-sharedlib")
        .expect("c sharedlib demo");
    test_build(&tmp_dir, &bin_path, "lua-demo", "lua").expect("lua demo");
    test_build(&tmp_dir, &bin_path, "lua-embedded-demo", "lua-embedded")
        .expect("lua embedded demo");

    // clean
    fs::remove_dir_all(&tmp_dir).expect("remove dir");
}

fn test_build<P: AsRef<Path>>(
    dir: P,
    bin_path: &str,
    name: &str,
    template_type: &str,
) -> Result<(), Error> {
    let mut contract_path = PathBuf::new();
    contract_path.push(&dir);
    contract_path.push(name);
    println!("Creating {:?} ...", contract_path);
    let output = Command::new(bin_path)
        .current_dir(&dir)
        .arg("new")
        .arg(name)
        .arg("--template")
        .arg(template_type)
        .output()?;
    if !output.status.success() {
        panic!(
            "command crash, stderr {:?}",
            String::from_utf8(output.stderr)?
        );
    }
    println!("Building ...");
    let exit_code = Command::new("bash")
        .current_dir(&contract_path)
        .arg("-c")
        .arg(format!("{} build --host", bin_path))
        .spawn()?
        .wait()?;
    if !exit_code.success() {
        panic!("command crash, exit_code {:?}", exit_code.code());
    }
    println!("Run contract test ...");
    let exit_code = Command::new("bash")
        .current_dir(&contract_path)
        .arg("-c")
        .arg(format!("{} test", bin_path))
        .spawn()?
        .wait()?;
    if !exit_code.success() {
        panic!("command crash, exit_code {:?}", exit_code.code());
    }
    println!("Clean contract ...");
    let exit_code = Command::new("bash")
        .current_dir(&contract_path)
        .arg("-c")
        .arg(format!("{} clean", bin_path))
        .spawn()?
        .wait()?;
    if !exit_code.success() {
        panic!("command crash, exit_code {:?}", exit_code.code());
    }
    println!("Success!");
    Ok(())
}

fn test_build_sharedlib<P: AsRef<Path>>(
    dir: P,
    bin_path: &str,
    name: &str,
    template_type: &str,
) -> Result<(), Error> {
    let mut contract_path = PathBuf::new();
    contract_path.push(&dir);
    contract_path.push(name);
    println!("Creating {:?} ...", contract_path);
    let exit_code = Command::new(bin_path)
        .current_dir(&dir)
        .arg("new")
        .arg(name)
        .arg("--template")
        .arg(template_type)
        .spawn()?
        .wait()?;
    if !exit_code.success() {
        panic!("command crash, exit_code {:?}", exit_code.code());
    }
    println!("Building ...");
    let exit_code = Command::new("bash")
        .current_dir(&contract_path)
        .arg("-c")
        .arg(format!("{} build --host", bin_path))
        .spawn()?
        .wait()?;
    if !exit_code.success() {
        panic!("command crash, exit_code {:?}", exit_code.code());
    }
    println!("Check shared library binary ...");
    let mut bin_path = contract_path.clone();
    bin_path.push(format!("build/debug/{}.so", name));
    if !bin_path.exists() {
        panic!("can't find shared library {:?}", bin_path);
    }
    println!("Success!");
    Ok(())
}
