use rodio::Decoder;

/// Sound ressource which can be loaded from a static source using [include_bytes!](include_bytes!)
pub struct Sound(pub &'static [u8]);

impl Sound {
    pub fn new(sound: &'static [u8]) -> Sound {
        return Self(sound);
    }

    /// Decode the sound so it can be played by a [sink](crate::audio::AudioSink).
    pub fn decode(&self) -> Decoder<std::io::Cursor<&'static [u8]>> {
        let cursor = std::io::Cursor::new(self.0);
        Decoder::new(cursor).unwrap()
    }
}

#[no_mangle]
pub extern "C" fn __cxa_pure_virtual() {
    loop {}
}
