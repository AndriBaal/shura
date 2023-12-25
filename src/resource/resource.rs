use anyhow::Result;
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use std::{env, fs, path::PathBuf};

#[cfg(feature = "log")]
use crate::log::info;

#[macro_export]
macro_rules! include_bytes_res {
    ($file:expr $(,)?) => {
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/", $file))
    };
}

#[macro_export]
macro_rules! include_str_res {
    ($file:expr $(,)?) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/", $file))
    };
}

#[macro_export]
macro_rules! include_wgsl_res {
    ($file:expr $(,)?) => {
        ::shura::include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/", $file))
    };
}

pub async fn load_res_bytes_async(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    {
        let url = resource_url(path)?;
        return Ok(reqwest::get(url).await?.bytes().await?.to_vec());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = resource_path(path)?;
        let data = std::fs::read(path)?;
        Ok(data)
    }
}

pub async fn load_res_string_async(path: impl AsRef<Path>) -> Result<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let url = resource_url(path)?;
        let test = reqwest::get(url).await?;
        let text = test.text().await?;
        return Ok(text);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = resource_path(path)?;
        let data = std::fs::read_to_string(path)?;
        Ok(data)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_res_bytes(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let path = resource_path(path)?;
    let data = std::fs::read(path)?;
    Ok(data)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_res_string(path: impl AsRef<Path>) -> Result<String> {
    let path = resource_path(path)?;
    let data = std::fs::read_to_string(path)?;
    Ok(data)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_data_bytes(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let path = data_path(path)?;
    let data = std::fs::read(path)?;
    Ok(data)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_data_string(path: impl AsRef<Path>) -> Result<String> {
    let path = data_path(path)?;
    let data = std::fs::read_to_string(path)?;
    Ok(data)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_data(path: impl AsRef<Path>, data: impl AsRef<[u8]>) -> Result<()> {
    let path = data_path(path)?;
    let prefix = path.parent().unwrap();
    if !prefix.exists() {
        std::fs::create_dir_all(prefix)?;
    }
    let r = std::fs::write(path, data)?;
    Ok(r)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn data_path(path: impl AsRef<Path>) -> Result<PathBuf> {
    let exe = env::current_exe()?;
    let mut dir = fs::canonicalize(exe)?;
    dir.pop();
    let path = dir.join("data").join(path);
    #[cfg(feature = "log")]
    info!("Saving data: {}", path.display());
    Ok(path)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn resource_path(path: impl AsRef<Path>) -> Result<PathBuf> {
    let exe = env::current_exe()?;
    let mut dir = fs::canonicalize(exe)?;
    dir.pop();
    let path = dir.join("res").join(path);
    #[cfg(feature = "log")]
    info!("Loading: {}", path.display());
    Ok(path)
}

#[cfg(target_arch = "wasm32")]
pub fn resource_url(path: impl AsRef<Path>) -> Result<reqwest::Url> {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let origin = location.origin().unwrap();
    let base = reqwest::Url::parse(&origin)?;
    let path_str = path.as_ref().to_str().unwrap();
    let url = base.join("res/")?.join(path_str)?;
    #[cfg(feature = "log")]
    info!("Loading: {}", url);
    return Ok(url);
}

#[cfg(target_arch = "wasm32")]
pub async fn get_bytes(url: reqwest::Url) -> Result<Vec<u8>> {
    let data = reqwest::get(url).await?;
    let bytes = data.bytes().await?;
    return Ok(bytes.to_vec());
}

#[cfg(target_arch = "wasm32")]
pub async fn get_text(url: reqwest::Url) -> Result<String> {
    let data = reqwest::get(url).await?;
    let text = data.text().await?;
    return Ok(text);
}
