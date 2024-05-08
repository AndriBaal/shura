use anyhow::Result;
use downcast_rs::{impl_downcast, Downcast};

use std::{env, fs, path::PathBuf};

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

pub trait AssetManager: Send + Sync + Downcast {
    fn load_bytes(&self, path: &str) -> Result<Vec<u8>>;
    fn load_string(&self, path: &str) -> Result<String>;
}
impl_downcast!(AssetManager);

pub trait StorageManager: Send + Sync + Downcast {
    fn store(&self, path: &str, data: &dyn AsRef<[u8]>) -> Result<()>;
    fn load_string(&self, path: &str) -> Result<String>;
    fn load_bytes(&self, path: &str) -> Result<Vec<u8>>;
}
impl_downcast!(StorageManager);

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

impl AssetManager for NativeAssetManager {
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
pub struct NativeStorageManager;
impl NativeStorageManager {
    fn data_path(&self, path: &str) -> Result<PathBuf> {
        let exe = env::current_exe()?;
        let mut dir = fs::canonicalize(exe)?;
        dir.pop();
        let path = dir.join("data").join(path);
        Ok(path)
    }
}

impl StorageManager for NativeStorageManager {
    fn store(&self, path: &str, data: &dyn AsRef<[u8]>) -> Result<()> {
        let path = self.data_path(path)?;
        let prefix = path.parent().unwrap();
        if !prefix.exists() {
            std::fs::create_dir_all(prefix)?;
        }
        std::fs::write(path, data)?;
        Ok(())
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
impl AssetManager for WebAssetManager {
    fn load_bytes(&self, path: &str) -> Result<Vec<u8>> {
        let url = self.asset_url(path)?;
        let response = reqwest::get(url)?;
        let bytes = response.bytes()?;
        return Ok(bytes.to_vec());
    }

    fn load_string(&self, path: &str) -> Result<String> {
        let url = self.asset_url(path)?;
        let response = reqwest::get(url)?;
        let text = response.text()?;
        return Ok(text);
    }
}

#[non_exhaustive]
#[cfg(target_arch = "wasm32")]
pub struct UnimplmentedStorageManager;

#[cfg(target_arch = "wasm32")]
impl StorageManager for UnimplmentedStorageManager {
    fn store(&self, _path: &str, _data: &dyn AsRef<[u8]>) -> Result<()> {
        unimplemented!()
    }

    fn load_string(&self, _path: &str) -> Result<String> {
        unimplemented!()
    }

    fn load_bytes(&self, _path: &str) -> Result<Vec<u8>> {
        unimplemented!()
    }
}

// #[non_exhaustive]
// #[cfg(target_arch = "wasm32")]
// pub struct WasmAssetManager;

// #[cfg(target_os = "android")]
// pub struct AndroidAssetManager {
//     android_assets: ndk::asset::AssetManager,
//     android_DATA: PathBuf,
// }

// pub async fn load_asset_bytes_async(path: &str) -> Result<Vec<u8>> {
//     #[cfg(target_arch = "wasm32")]
//     {
//         let url = asset_url(path)?;
//         return Ok(reqwest::get(url).await?.bytes().await?.to_vec());
//     }
//     #[cfg(not(target_arch = "wasm32"))]
//     {
//         load_asset_bytes(path)
//     }
// }

// pub async fn load_asset_string_async(path: &str) -> Result<String> {
//     #[cfg(target_arch = "wasm32")]
//     {
//         let url = asset_url(path)?;
//         let test = reqwest::get(url).await?;
//         let text = test.text().await?;
//         return Ok(text);
//     }
//     #[cfg(not(target_arch = "wasm32"))]
//     {
//         load_asset_string(path)
//     }
// }

// #[cfg(not(target_arch = "wasm32"))]
// pub fn load_asset_bytes(path: &str) -> Result<Vec<u8>> {
//     #[cfg(target_os = "android")]
//     {
//         #[cfg(feature = "log")]
//         info!("Loading: {}", path);
//         let manager = ANDROID_ASSETS.get().unwrap();
//         let path = CString::new(path).unwrap();
//         let mut asset = manager.open(&path).unwrap();
//         let mut data = vec![];
//         asset.read_to_end(&mut data).unwrap();
//         return Ok(data);
//     }
//     #[cfg(not(target_os = "android"))]
//     {
//         let path = asset_path(path)?;
//         let data = std::fs::read(path)?;
//         Ok(data)
//     }
// }

// #[cfg(not(target_arch = "wasm32"))]
// pub fn load_asset_string(path: &str) -> Result<String> {
//     #[cfg(target_os = "android")]
//     {
//         #[cfg(feature = "log")]
//         info!("Loading: {}", path);
//         let manager = ANDROID_ASSETS.get().unwrap();
//         let path = CString::new(path).unwrap();
//         let mut asset = manager.open(&path).unwrap();
//         let mut data = String::new();
//         asset.read_to_string(&mut data).unwrap();
//         return Ok(data);
//     }
//     #[cfg(not(target_os = "android"))]
//     {
//         let path = asset_path(path)?;
//         let data = std::fs::read_to_string(path)?;
//         Ok(data)
//     }
// }

// #[cfg(not(target_arch = "wasm32"))]
// pub fn load_data_bytes(path: &str) -> Result<Vec<u8>> {
//     #[cfg(target_arch = "wasm32")]
//     {
//         todo!()
//     }
//     #[cfg(target_os = "android")]
//     {
//         todo!()
//     }
//     #[cfg(not(target_os = "android"))]
//     {
//         let path = data_path(path)?;
//         let data = std::fs::read(path)?;
//         Ok(data)
//     }
// }

// #[cfg(not(target_arch = "wasm32"))]
// pub fn load_data_string(path: &str) -> Result<String> {
//     #[cfg(target_arch = "wasm32")]
//     {
//         todo!()
//     }
//     #[cfg(target_os = "android")]
//     {
//         todo!()
//     }
//     #[cfg(not(target_os = "android"))]
//     {
//         let path = data_path(path)?;
//         let data = std::fs::read_to_string(path)?;
//         Ok(data)
//     }
// }

// #[cfg(not(target_arch = "wasm32"))]
// pub fn delete_data(path: &str) -> Result<()> {
//     #[cfg(target_arch = "wasm32")]
//     {
//         todo!()
//     }
//     #[cfg(target_os = "android")]
//     {
//         todo!()
//     }
//     #[cfg(not(target_os = "android"))]
//     {
//         let path = data_path(path)?;
//         if path.is_dir() {
//             std::fs::remove_dir_all(path)?;
//         } else {
//             std::fs::remove_file(path)?;
//         }
//         Ok(())
//     }
// }

// // #[cfg(not(target_arch = "wasm32"))]
// // pub fn read_data_dir(path: &str) -> Result<std::fs::ReadDir> {
// //     #[cfg(target_arch = "wasm32")]
// //     {
// //         todo!()
// //     }
// //     #[cfg(target_os = "android")]
// //     {
// //         todo!()
// //     }
// //     #[cfg(not(target_os = "android"))]
// //     {
// //         let path = data_path(path)?;
// //         let data = std::fs::read_dir(path)?;
// //         Ok(data)
// //     }
// // }

// #[cfg(not(target_arch = "wasm32"))]
// pub fn store(path: &str, data: impl AsRef<[u8]>) -> Result<()> {
//     #[cfg(target_os = "android")]
//     {
//         todo!()
//     }
//     #[cfg(not(target_os = "android"))]
//     {
//         let path = data_path(path)?;
//         let prefix = path.parent().unwrap();
//         if !prefix.exists() {
//             std::fs::create_dir_all(prefix)?;
//         }
//         std::fs::write(path, data)?;
//         Ok(())
//     }
// }

// #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
// pub fn data_path(path: &str) -> Result<PathBuf> {
//     let exe = env::current_exe()?;
//     let mut dir = fs::canonicalize(exe)?;
//     dir.pop();
//     let path = dir.join("data").join(path);
//     Ok(path)
// }

// #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
// pub fn asset_path(path: &str) -> Result<PathBuf> {
//     let exe = env::current_exe()?;
//     let mut dir = fs::canonicalize(exe)?;
//     dir.pop();
//     let path = dir.join("assets").join(path);
//     #[cfg(feature = "log")]
//     info!("Loading: {}", path.display());
//     Ok(path)
// }

// #[cfg(target_arch = "wasm32")]
// pub fn asset_url(path: &str) -> Result<reqwest::Url> {
//     let window = web_sys::window().unwrap();
//     let location = window.location();
//     let origin = location.origin().unwrap();
//     let base = reqwest::Url::parse(&origin)?;
//     let url = base.join("assets/")?.join(path)?;
//     #[cfg(feature = "log")]
//     info!("Loading: {}", url);
//     return Ok(url);
// }
