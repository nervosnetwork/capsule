use std::fs;
use std::env;
use std::path::PathBuf;
use ckb_tool::ckb_types::bytes::Bytes;

#[cfg(test)]
mod tests;

pub struct Loader(PathBuf);

impl Default for Loader {
    fn default() -> Self {
        let dir = env::current_dir().unwrap();
        let mut base_path = PathBuf::new();
        base_path.push(dir);
        base_path.push("..");
        base_path.push("build");
        Loader(base_path)
    }
}

impl Loader {
    pub fn load_binary(&self, name: &str) -> Bytes {
        let mut path = self.0.clone();
        path.push(name);
        fs::read(path).expect("binary").into()
    }
}
