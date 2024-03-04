use crate::wrapper::audio_processor::cross_fader::CrossFader;
use crate::wrapper::extensions::PluginAudioPortsInfo;
use clack_host::prelude::{AudioPorts, OutputAudioBuffers};
use clack_plugin::prelude::Audio;

// TODO: handle 64bit buffers
pub struct OutputBuffers {
    main_buffers: Vec<Vec<f32>>, // 1 per channel per port
    fading_out_buffers: Vec<Vec<f32>>,
    audio_port_buffers: AudioPorts,
}

impl OutputBuffers {
    pub fn new_from_config(info: &PluginAudioPortsInfo) -> Self {
        todo!()
    }

    pub fn output_buffers_for(&mut self, get_main: bool) -> OutputAudioBuffers {
        todo!()
    }

    pub fn output_crossfade(&mut self, cross_fader: &mut CrossFader, audio: &mut Audio) {
        todo!()
    }
}
