use crate::audio::{AudioSink, Sound, SoundBuilder};

pub struct AudioDeviceManager {
    pub output_stream: rodio::OutputStream,
    pub output_handle: rodio::OutputStreamHandle,
}

impl AudioDeviceManager {
    pub(crate) fn new() -> (Self, AudioManager) {
        let (output_stream, output_handle) = rodio::OutputStream::try_default().unwrap();
        (
            Self {
                output_stream,
                output_handle: output_handle.clone(),
            },
            AudioManager { output_handle },
        )
    }

    // TODO: Custom device
}

#[derive(Clone)]
pub struct AudioManager {
    pub output_handle: rodio::OutputStreamHandle,
}

impl AudioManager {
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
