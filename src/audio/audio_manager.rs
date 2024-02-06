use crate::audio::{AudioSink, Sound, SoundBuilder};

// Thin wrapper around rodio
pub struct AudioManager {
    pub output_stream: rodio::OutputStream,
    pub output_handle: rodio::OutputStreamHandle,
}

impl AudioManager {
    pub(crate) fn new() -> Self {
        let (output_stream, output_handle) = rodio::OutputStream::try_default().unwrap();
        Self {
            output_stream,
            output_handle,
        }
    }

    pub fn play_once(&self, sound: &Sound) {
        self.output_handle
            .play_once(sound.cursor())
            .unwrap()
            .detach()
    }

    pub fn play_once_and(&self, sound: &Sound) -> AudioSink {
        self.output_handle.play_once(sound.cursor()).unwrap()
    }

    pub fn create_sink(&self) -> AudioSink {
        AudioSink::try_new(&self.output_handle).unwrap()
    }

    pub fn create_sound(&self, builder: SoundBuilder) -> Sound {
        Sound::new(builder)
    }
}
