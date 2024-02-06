use crate::prelude::load_asset_bytes_async;
use rodio::Decoder;
use std::sync::Arc;

pub struct SoundBuilder {
    data: Vec<u8>,
}

impl SoundBuilder {
    pub fn bytes(sound: &[u8]) -> Self {
        Self {
            data: sound.to_vec(),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn asset(path: &str) -> Self {
        use crate::prelude::load_asset_bytes;
        let bytes = load_asset_bytes(path).unwrap();
        Self::bytes(&bytes)
    }

    pub async fn asset_async(path: &str) -> Self {
        let bytes = load_asset_bytes_async(path).await.unwrap();
        Self::bytes(&bytes)
    }
}

#[derive(Clone)]
pub struct Sound(pub Arc<Vec<u8>>);
impl AsRef<[u8]> for Sound {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Sound {
    pub fn new(builder: SoundBuilder) -> Self {
        Self(Arc::new(builder.data))
    }

    pub fn decode(&self) -> Decoder<std::io::Cursor<Self>> {
        Decoder::new(self.cursor()).unwrap()
    }

    pub fn cursor(&self) -> std::io::Cursor<Self> {
        std::io::Cursor::new(self.clone())
    }
}

#[no_mangle]
#[allow(clippy::empty_loop)]
pub extern "C" fn __cxa_pure_virtual() {
    loop {}
}
