use crate::audio::{PlayError, Sink, Sound};

pub struct AudioManager {
    pub output_stream: rodio::OutputStream,
    pub output_handle: rodio::OutputStreamHandle,
}

impl AudioManager {
    pub fn new() -> Self {
        let (output_stream, output_handle) = rodio::OutputStream::try_default().unwrap();
        return Self {
            output_stream,
            output_handle,
        };
    }

    pub fn create_sink(&self) -> Sink {
        let s = Sink::try_new(&self.output_handle).unwrap();
        Sink::try_new(&self.output_handle).unwrap()
    }

    pub fn create_sound(&self, bytes: &'static [u8]) -> Sound {
        return Sound::new(bytes);
    }
}
