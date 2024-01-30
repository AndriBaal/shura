use anyhow::Result;

#[cfg(not(target_arch = "wasm32"))]
use std::{env, fs, path::PathBuf};

#[cfg(feature = "log")]
use crate::log::info;

#[cfg(target_os = "android")]
use std::{sync::OnceLock, ffi::CString, io::Read};

#[cfg(target_os = "android")]
pub(crate) static ANDROID_ASSETS: OnceLock<ndk::asset::AssetManager> = OnceLock::new();

#[cfg(target_os = "android")]
pub(crate) static ANDROID_DATA: OnceLock<PathBuf> = OnceLock::new();

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

pub async fn load_res_bytes_async(path: &str) -> Result<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    {
        let url = resource_url(path)?;
        return Ok(reqwest::get(url).await?.bytes().await?.to_vec());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        load_res_bytes(path)
    }
}

pub async fn load_res_string_async(path: &str) -> Result<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let url = resource_url(path)?;
        let test = reqwest::get(url).await?;
        let text = test.text().await?;
        return Ok(text);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        load_res_string(path)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_res_bytes(path: &str) -> Result<Vec<u8>> {
    #[cfg(target_os = "android")] {
        #[cfg(feature = "log")]
        info!("Loading: {}", path);
        let manager = ANDROID_ASSETS.get().unwrap();
        let path = CString::new(path).unwrap();
        let mut asset = manager.open(&path).unwrap();
        let mut data = vec![];
        asset.read_to_end(&mut data).unwrap();
        return Ok(data);
    }
    #[cfg(not(target_os = "android"))] {
        let path = resource_path(path)?;
        let data = std::fs::read(path)?;
        Ok(data)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_res_string(path: &str) -> Result<String> {
    #[cfg(target_os = "android")] {
        #[cfg(feature = "log")]
        info!("Loading: {}", path);
        let manager = ANDROID_ASSETS.get().unwrap();
        let path = CString::new(path).unwrap();
        let mut asset = manager.open(&path).unwrap();
        let mut data = String::new();
        asset.read_to_string(&mut data).unwrap();
        return Ok(data);
    }
    #[cfg(not(target_os = "android"))] {
        let path = resource_path(path)?;
        let data = std::fs::read_to_string(path)?;
        Ok(data)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_data_bytes(path: &str) -> Result<Vec<u8>> {
    #[cfg(target_os = "android")] {
        todo!()
    }
    #[cfg(not(target_os = "android"))] {
        let path = data_path(path)?;
        let data = std::fs::read(path)?;
        Ok(data)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_data_string(path: &str) -> Result<String> {
    #[cfg(target_os = "android")] {
        todo!()
    }
    #[cfg(not(target_os = "android"))] {
        let path = data_path(path)?;
        let data = std::fs::read_to_string(path)?;
        Ok(data)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_data(path: &str, data: impl AsRef<[u8]>) -> Result<()> {
    #[cfg(target_os = "android")] {
        todo!()
    }
    #[cfg(not(target_os = "android"))] {
        let path = data_path(path)?;
        let prefix = path.parent().unwrap();
        if !prefix.exists() {
            std::fs::create_dir_all(prefix)?;
        }
        std::fs::write(path, data)?;
        Ok(())
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os="android")))]
pub fn data_path(path: &str) -> Result<PathBuf> {
    let exe = env::current_exe()?;
    let mut dir = fs::canonicalize(exe)?;
    dir.pop();
    let path = dir.join("data").join(path);
    #[cfg(feature = "log")]
    info!("Saving data: {}", path.display());
    Ok(path)
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os="android")))]
pub fn resource_path(path: &str) -> Result<PathBuf> {
    let exe = env::current_exe()?;
    let mut dir = fs::canonicalize(exe)?;
    dir.pop();
    let path = dir.join("res").join(path);
    #[cfg(feature = "log")]
    info!("Loading: {}", path.display());
    Ok(path)
}

#[cfg(target_arch = "wasm32")]
pub fn resource_url(path: &str) -> Result<reqwest::Url> {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let origin = location.origin().unwrap();
    let base = reqwest::Url::parse(&origin)?;
    let url = base.join("res/")?.join(path)?;
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
