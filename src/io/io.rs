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
macro_rules! include_resource_bytes {
    ($file:expr $(,)?) => {
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/", $file))
    };
}

#[macro_export]
macro_rules! include_resource_str {
    ($file:expr $(,)?) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/", $file))
    };
}

#[macro_export]
macro_rules! include_resource_wgsl {
    ($file:expr $(,)?) => {
        ::shura::graphics::include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/", $file))
    };
}

pub static GLOBAL_RESOURCE_LOADER: OnceLock<Arc<dyn ResourceLoader>> = OnceLock::new();
pub static GLOBAL_STORAGE_LOADER: OnceLock<Arc<dyn StorageLoader>> = OnceLock::new();

#[async_trait::async_trait(?Send)]
pub trait ResourceLoader: Send + Sync + Downcast {
    fn load_bytes(&self, path: &str) -> Result<Vec<u8>>;
    fn load_string(&self, path: &str) -> Result<String>;
    async fn async_load_bytes(&self, path: &str) -> Result<Vec<u8>> {
        self.load_bytes(path)
    }
    async fn async_load_string(&self, path: &str) -> Result<String> {
        self.load_string(path)
    }
}
impl_downcast!(ResourceLoader);

pub trait StorageLoader: Send + Sync + Downcast {
    fn store(&self, path: &str, data: &dyn AsRef<[u8]>) -> Result<()>;
    fn load_string(&self, path: &str) -> Result<String>;
    fn delete(&self, path: &str) -> Result<()>;
    fn load_bytes(&self, path: &str) -> Result<Vec<u8>>;
    fn list(&self) -> Vec<String>;
}
impl_downcast!(StorageLoader);

#[non_exhaustive]
pub struct NativeResourceLoader {
    pub resource_dir: PathBuf,
}

impl NativeResourceLoader {
    pub fn new() -> Result<Self> {
        let exe = env::current_exe()?;
        let mut dir = fs::canonicalize(exe)?;
        dir.pop();
        let resource_dir = dir.join("resources");
        Ok(Self { resource_dir })
    }
}

impl NativeResourceLoader {
    fn resource_path(&self, path: &str) -> PathBuf {
        self.resource_dir.join(path)
    }
}

impl ResourceLoader for NativeResourceLoader {
    fn load_bytes(&self, path: &str) -> Result<Vec<u8>> {
        let path = self.resource_path(path);
        let data = std::fs::read(path)?;
        Ok(data)
    }

    fn load_string(&self, path: &str) -> Result<String> {
        let path = self.resource_path(path);
        let data = std::fs::read_to_string(path)?;
        Ok(data)
    }
}

#[non_exhaustive]
pub struct NativeStorageLoader {
    pub data_dir: PathBuf,
}

impl NativeStorageLoader {
    pub fn new() -> Result<Self> {
        let exe = env::current_exe()?;
        let mut dir = fs::canonicalize(exe)?;
        dir.pop();
        let data_dir = dir.join("data");
        Ok(Self { data_dir })
    }
}

impl NativeStorageLoader {
    fn data_path(&self, path: &str) -> PathBuf {
        self.data_dir.join(path)
    }
}

impl StorageLoader for NativeStorageLoader {
    fn store(&self, path: &str, data: &dyn AsRef<[u8]>) -> Result<()> {
        let path = self.data_path(path);
        let prefix = path.parent().unwrap();
        if !prefix.exists() {
            std::fs::create_dir_all(prefix)?;
        }
        Ok(std::fs::write(path, data)?)
    }

    fn load_string(&self, path: &str) -> Result<String> {
        let path = self.data_path(path);
        let data = std::fs::read_to_string(path)?;
        Ok(data)
    }

    fn load_bytes(&self, path: &str) -> Result<Vec<u8>> {
        let path = self.data_path(path);
        let data = std::fs::read(path)?;
        Ok(data)
    }

    fn list(&self) -> Vec<String> {
        std::fs::read_dir(&self.data_dir)
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
        let path = self.data_path(path);
        Ok(std::fs::remove_file(path)?)
    }
}

#[cfg(target_arch = "wasm32")]
#[non_exhaustive]
pub struct WebResourceLoader;

#[cfg(target_arch = "wasm32")]
impl WebResourceLoader {
    pub fn resource_url(&self, path: &str) -> Result<reqwest::Url> {
        let window = web_sys::window().unwrap();
        let location = window.location();
        let origin = location.origin().unwrap();
        let base = reqwest::Url::parse(&origin)?;
        let url = base.join("resources/")?.join(path)?;
        return Ok(url);
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait::async_trait(?Send)]
impl ResourceLoader for WebResourceLoader {
    fn load_bytes(&self, _path: &str) -> Result<Vec<u8>> {
        unimplemented!("Synchronous asset operations are not allowed with WASM!")
    }

    fn load_string(&self, _path: &str) -> Result<String> {
        unimplemented!("Synchronous asset operations are not allowed with WASM!")
    }

    async fn async_load_bytes(&self, path: &str) -> Result<Vec<u8>> {
        let url = self.resource_url(path)?;
        let response = reqwest::get(url).await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    async fn async_load_string(&self, path: &str) -> Result<String> {
        let url = self.resource_url(path)?;
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
pub struct AndroidResourceLoader {
    manager: ndk::asset::ResourceLoader,
}

#[cfg(target_os = "android")]
impl AndroidResourceLoader {
    pub fn new(app: &winit::platform::android::activity::AndroidApp) -> Self {
        Self {
            manager: app.asset_manager(),
        }
    }
}

#[cfg(target_os = "android")]
impl ResourceLoader for AndroidResourceLoader {
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
