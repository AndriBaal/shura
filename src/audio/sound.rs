use rodio::Decoder;
use std::sync::Arc;

use crate::io::AssetManager;

#[derive(Clone)]
pub struct SoundBuilder {
    data: Vec<u8>,
}

impl SoundBuilder {
    pub fn bytes(sound: &[u8]) -> Self {
        Self {
            data: sound.to_vec(),
        }
    }

    pub fn asset(assets: &dyn AssetManager, path: &str) -> Self {
        let bytes = assets.load_bytes(path).unwrap();
        Self::bytes(&bytes)
    }
}

impl From<SoundBuilder> for Sound {
    fn from(val: SoundBuilder) -> Self {
        Sound::new(val)
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
