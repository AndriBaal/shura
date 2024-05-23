use anyhow::Result;
use downcast_rs::{impl_downcast, Downcast};

use std::{env, fs, path::PathBuf};

#[cfg(target_os = "android")]
use std::{ffi::CString, io::Read};

#[cfg(feature = "audio")]
use crate::audio::SoundBuilder;

#[cfg(feature = "text")]
use crate::text::FontBuilder;

use crate::graphics::{ModelBuilder, SpriteArrayBuilder, SpriteBuilder, TileSize};

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

#[async_trait::async_trait(?Send)]
pub trait BaseAssetManager: Send + Sync + Downcast {
    fn load_bytes(&self, path: &str) -> Result<Vec<u8>>;
    fn load_string(&self, path: &str) -> Result<String>;
    async fn async_load_bytes(&self, path: &str) -> Result<Vec<u8>> {
        self.load_bytes(path)
    }
    async fn async_load_string(&self, path: &str) -> Result<String> {
        self.load_string(path)
    }
}
impl_downcast!(BaseAssetManager);

pub trait AssetManager: BaseAssetManager {
    fn load_sprite_array_sheet(
        &self,
        path: &str,
        size: TileSize,
    ) -> SpriteArrayBuilder<image::RgbaImage>;
    fn load_sprite_array(&self, paths: &[&str]) -> SpriteArrayBuilder<image::RgbaImage>;
    #[cfg(feature = "audio")]
    fn load_sound(&self, path: &str) -> SoundBuilder;
    #[cfg(feature = "text")]
    fn load_font(&self, path: &str) -> FontBuilder;
    fn load_model(&self, path: &str) -> ModelBuilder;
    fn load_sprite(&self, path: &str) -> SpriteBuilder<image::RgbaImage>;
    // async fn async_load_sprite_array_sheet(
    //     &self,
    //     path: &str,
    //     size: TileSize,
    // ) -> SpriteArrayBuilder<image::RgbaImage>;
    // async fn async_load_sprite_array(&self, paths: &[&str]) -> SpriteArrayBuilder<image::RgbaImage>;
    // #[cfg(feature = "audio")]
    // async fn async_load_sound(&self, path: &str) -> SoundBuilder;
    // #[cfg(feature = "text")]
    // async fn async_load_font(&self, path: &str) -> FontBuilder;
    // async fn async_load_model(&self, path: &str) -> ModelBuilder;
    // async fn async_load_sprite(&self, path: &str) -> SpriteBuilder<image::RgbaImage>;
}

impl<A: BaseAssetManager> AssetManager for A {
    fn load_sprite_array_sheet(
        &self,
        path: &str,
        size: TileSize,
    ) -> SpriteArrayBuilder<image::RgbaImage> {
        SpriteArrayBuilder::asset_sheet(self, path, size)
    }
    fn load_sprite_array(&self, paths: &[&str]) -> SpriteArrayBuilder<image::RgbaImage> {
        SpriteArrayBuilder::assets(self, paths)
    }
    #[cfg(feature = "audio")]
    fn load_sound(&self, path: &str) -> SoundBuilder {
        SoundBuilder::asset(self, path)
    }
    #[cfg(feature = "text")]
    fn load_font(&self, path: &str) -> FontBuilder {
        FontBuilder::asset(self, path)
    }
    fn load_model(&self, path: &str) -> ModelBuilder {
        ModelBuilder::asset(self, path)
    }
    fn load_sprite(&self, path: &str) -> SpriteBuilder<image::RgbaImage> {
        SpriteBuilder::asset(self, path)
    }
}

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

impl BaseAssetManager for NativeAssetManager {
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
#[async_trait::async_trait(?Send)]
impl BaseAssetManager for WebAssetManager {
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
#[cfg(target_arch = "wasm32")]
pub struct UnimplementedStorageManager;

#[cfg(target_arch = "wasm32")]
impl StorageManager for UnimplementedStorageManager {
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

#[cfg(target_os = "android")]
pub struct AndroidAssetManager {
    manager: ndk::asset::AssetManager,
}

#[cfg(target_os = "android")]
impl AndroidAssetManager {
    pub fn new(app: &winit::platform::android::activity::AndroidApp) -> Self {
        Self {
            manager: app.asset_manager()
        }
    }
}

#[cfg(target_os = "android")]
impl BaseAssetManager for AndroidAssetManager {
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

// pub struct AndroidStorageManager {
//     android_data: PathBuf,
// }

// impl AndroidStorageManager {
//     pub fn new(app: &winit::platform::android::activity::AndroidApp) -> Self {
//         Self {
//             manager: app.internal_data_path().unwrap()
//         }
//     }

//     fn data_path(&self, path: &str) -> Result<PathBuf> {
//         let path = self.android_data.join(path);
//         Ok(path)
//     }
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
