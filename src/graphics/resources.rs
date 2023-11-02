
use std::path::Path;
use std::{env, fs, path::PathBuf};
use std::io::Result;


#[cfg(feature="log")]
use crate::log::info;

pub fn load_bytes(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let path = resource_path(path)?;
    return std::fs::read(path);
}

pub fn load_string(path: impl AsRef<Path>) -> Result<String> {
    let path = resource_path(path)?;
    return std::fs::read_to_string(path);
}

pub fn resource_path(path: impl AsRef<Path>) -> Result<PathBuf> {
    let exe = env::current_exe()?;
    let mut dir = fs::canonicalize(exe)?;
    dir.pop();
    let path = dir.join("res").join(path);
    #[cfg(feature="log")]
    info!("Loading: {}", path.display());
    return Ok(path);
}

