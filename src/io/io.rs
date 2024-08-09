use anyhow::Result;
use downcast_rs::{impl_downcast, Downcast};

use std::{
    env, fs,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

#[cfg(target_os = "android")]
use std::{ffi::CString, io::Read};

#[macro_export]
macro_rules! include_asset_bytes {
    ($file:expr $(,)?) => {
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/", $file))
    };
}

#[macro_export]
macro_rules! include_asset_str {
    ($file:expr $(,)?) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/", $file))
    };
}

#[macro_export]
macro_rules! include_asset_wgsl {
    ($file:expr $(,)?) => {
        ::shura::graphics::include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/", $file))
    };
}

pub static GLOBAL_ASSET_LOADER: OnceLock<Arc<dyn AssetLoader>> = OnceLock::new();
pub static GLOBAL_STORAGE_LOADER: OnceLock<Arc<dyn StorageLoader>> = OnceLock::new();

#[async_trait::async_trait(?Send)]
pub trait AssetLoader: Send + Sync + Downcast {
    fn load_bytes(&self, path: &str) -> Result<Vec<u8>>;
    fn load_string(&self, path: &str) -> Result<String>;
    async fn async_load_bytes(&self, path: &str) -> Result<Vec<u8>> {
        self.load_bytes(path)
    }
    async fn async_load_string(&self, path: &str) -> Result<String> {
        self.load_string(path)
    }
}
impl_downcast!(AssetLoader);

pub trait StorageLoader: Send + Sync + Downcast {
    fn store(&self, path: &str, data: &dyn AsRef<[u8]>) -> Result<()>;
    fn load_string(&self, path: &str) -> Result<String>;
    fn delete(&self, path: &str) -> Result<()>;
    fn load_bytes(&self, path: &str) -> Result<Vec<u8>>;
    fn list(&self) -> Vec<String>;
}
impl_downcast!(StorageLoader);

// #[cfg(any(
//     target_os = "windows",
//     target_os = "macos",
//     target_os = "linux",
//     target_os = "freebsd",
//     target_os = "fuchsia",
//     target_os = "redox"
// ))] // I don't know if freebsd, fuchsia and redox even works
// Maybe move to build script
#[non_exhaustive]
pub struct NativeAssetManager;
impl NativeAssetManager {
    fn asset_path(&self, path: &str) -> Result<PathBuf> {
        let exe = env::current_exe()?;
        let mut dir = fs::canonicalize(exe)?;
        dir.pop();
        let path = dir.join("assets").join(path);
        Ok(path)
    }
}

impl AssetLoader for NativeAssetManager {
    fn load_bytes(&self, path: &str) -> Result<Vec<u8>> {
        let path = self.asset_path(path)?;
        let data = std::fs::read(path)?;
        Ok(data)
    }

    fn load_string(&self, path: &str) -> Result<String> {
        let path = self.asset_path(path)?;
        let data = std::fs::read_to_string(path)?;
        Ok(data)
    }
}

#[non_exhaustive]
pub struct NativeStorageLoader;
impl NativeStorageLoader {
    fn data_path(&self, path: &str) -> Result<PathBuf> {
        let exe = env::current_exe()?;
        let mut dir = fs::canonicalize(exe)?;
        dir.pop();
        let path = dir.join("data").join(path);
        Ok(path)
    }
}

impl StorageLoader for NativeStorageLoader {
    fn store(&self, path: &str, data: &dyn AsRef<[u8]>) -> Result<()> {
        let path = self.data_path(path)?;
        let prefix = path.parent().unwrap();
        if !prefix.exists() {
            std::fs::create_dir_all(prefix)?;
        }
        Ok(std::fs::write(path, data)?)
    }

    fn load_string(&self, path: &str) -> Result<String> {
        let path = self.data_path(path)?;
        let data = std::fs::read_to_string(path)?;
        Ok(data)
    }

    fn load_bytes(&self, path: &str) -> Result<Vec<u8>> {
        let path = self.data_path(path)?;
        let data = std::fs::read(path)?;
        Ok(data)
    }

    fn list(&self) -> Vec<String> {
        let path = self.data_path("/").unwrap();
        std::fs::read_dir(path)
            .unwrap()
            .filter_map(|f| {
                if let Ok(f) = f {
                    if let Some(file) = f.path().file_name() {
                        return file.to_os_string().into_string().ok();
                    }
                }
                None
            })
            .collect()
    }

    fn delete(&self, path: &str) -> Result<()> {
        let path = self.data_path(path)?;
        Ok(std::fs::remove_file(path)?)
    }
}

#[cfg(target_arch = "wasm32")]
#[non_exhaustive]
pub struct WebAssetManager;

#[cfg(target_arch = "wasm32")]
impl WebAssetManager {
    pub fn asset_url(&self, path: &str) -> Result<reqwest::Url> {
        let window = web_sys::window().unwrap();
        let location = window.location();
        let origin = location.origin().unwrap();
        let base = reqwest::Url::parse(&origin)?;
        let url = base.join("assets/")?.join(path)?;
        return Ok(url);
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait::async_trait(?Send)]
impl AssetLoader for WebAssetManager {
    fn load_bytes(&self, _path: &str) -> Result<Vec<u8>> {
        unimplemented!("Synchronous asset operations are not allowed with WASM!")
    }

    fn load_string(&self, _path: &str) -> Result<String> {
        unimplemented!("Synchronous asset operations are not allowed with WASM!")
    }

    async fn async_load_bytes(&self, path: &str) -> Result<Vec<u8>> {
        let url = self.asset_url(path)?;
        let response = reqwest::get(url).await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    async fn async_load_string(&self, path: &str) -> Result<String> {
        let url = self.asset_url(path)?;
        let response = reqwest::get(url).await?;
        let text = response.text().await?;
        Ok(text)
    }
}

#[non_exhaustive]
pub struct UnimplementedStorageLoader;

impl StorageLoader for UnimplementedStorageLoader {
    fn store(&self, _path: &str, _data: &dyn AsRef<[u8]>) -> Result<()> {
        unimplemented!()
    }

    fn load_string(&self, _path: &str) -> Result<String> {
        unimplemented!()
    }

    fn load_bytes(&self, _path: &str) -> Result<Vec<u8>> {
        unimplemented!()
    }

    fn delete(&self, _path: &str) -> Result<()> {
        unimplemented!()
    }

    fn list(&self) -> Vec<String> {
        unimplemented!()
    }
}

#[cfg(target_os = "android")]
pub struct AndroidAssetManager {
    manager: ndk::asset::AssetManager,
}

#[cfg(target_os = "android")]
impl AndroidAssetManager {
    pub fn new(app: &winit::platform::android::activity::AndroidApp) -> Self {
        Self {
            manager: app.asset_manager(),
        }
    }
}

#[cfg(target_os = "android")]
impl AssetLoader for AndroidAssetManager {
    fn load_bytes(&self, path: &str) -> Result<Vec<u8>> {
        let manager = &self.manager;
        let path = CString::new(path).unwrap();
        let mut asset = manager.open(&path).unwrap();
        let mut data = vec![];
        asset.read_to_end(&mut data).unwrap();
        return Ok(data);
    }

    fn load_string(&self, path: &str) -> Result<String> {
        let manager = &self.manager;
        let path = CString::new(path).unwrap();
        let mut asset = manager.open(&path).unwrap();
        let mut data = String::new();
        asset.read_to_string(&mut data).unwrap();
        return Ok(data);
    }
}
