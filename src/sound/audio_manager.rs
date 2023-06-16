use crate::audio::{AudioSink, Sound};

// Thin wrapper around rodio
pub struct AudioManager {
    pub output_stream: rodio::OutputStream,
    pub output_handle: rodio::OutputStreamHandle,
}

impl AudioManager {
    pub(crate) fn new() -> Self {
        let (output_stream, output_handle) = rodio::OutputStream::try_default().unwrap();
        return Self {
            output_stream,
            output_handle,
        };
    }

    pub fn play_once(&self, sound: &Sound) {
        self.output_handle
            .play_once(std::io::Cursor::new(sound.0))
            .unwrap()
            .detach()
    }

    pub fn play_once_and(&self, sound: &Sound) -> AudioSink {
        self.output_handle
            .play_once(std::io::Cursor::new(sound.0))
            .unwrap()
    }

    pub fn create_sink(&self) -> AudioSink {
        AudioSink::try_new(&self.output_handle).unwrap()
    }

    pub fn create_sound(&self, bytes: &'static [u8]) -> Sound {
        return Sound::new(bytes);
    }
}
