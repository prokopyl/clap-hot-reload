use crate::wrapper::audio_processor::cross_fader::CrossFader;
use crate::wrapper::extensions::PluginAudioPortsInfo;
use clack_host::prelude::{AudioPortBuffer, AudioPortBufferType, AudioPorts, OutputAudioBuffers};
use clack_plugin::prelude::{Audio, AudioConfiguration, PluginError};

// TODO: handle 64bit buffers
pub struct OutputBuffers {
    main_buffers: Vec<Vec<Vec<f32>>>, // 1 per channel per port
    fading_out_buffers: Vec<Vec<Vec<f32>>>,
    audio_port_buffers: AudioPorts,
    buffer_frame_count: u32,
}

impl OutputBuffers {
    pub fn new_from_config(
        info: &PluginAudioPortsInfo,
        audio_configuration: AudioConfiguration,
    ) -> Self {
        let buffer_frame_count = audio_configuration.max_sample_count;
        let channel_count_per_port = info.output_channels_count_per_port();

        let port_count = channel_count_per_port.len();
        let total_channel_count: u32 = channel_count_per_port.iter().sum();

        let buffers: Vec<_> = channel_count_per_port
            .iter()
            .map(|channel_count| {
                vec![vec![0.0; buffer_frame_count as usize]; *channel_count as usize]
            })
            .collect();

        Self {
            fading_out_buffers: buffers.clone(),
            main_buffers: buffers,
            audio_port_buffers: AudioPorts::with_capacity(total_channel_count as usize, port_count),
            buffer_frame_count,
        }
    }

    pub fn output_buffers_for(&mut self, get_main: bool) -> OutputAudioBuffers {
        let bufs = if get_main {
            &mut self.main_buffers
        } else {
            &mut self.fading_out_buffers
        };

        self.audio_port_buffers
            // TODO: try and see if it works with AsRef instead of needing straight up slice
            .with_output_buffers(bufs.iter_mut().map(|buf| AudioPortBuffer {
                latency: 0, // TODO: handle latency by adding a PortInfo that only takes & from audio.
                channels: AudioPortBufferType::f32_output_only(
                    buf.iter_mut().map(|buf| buf.as_mut_slice()),
                ),
            }))
    }

    pub fn output_crossfade(
        &mut self,
        cross_fader: &mut CrossFader,
        audio: &mut Audio,
    ) -> Result<(), PluginError> {
        for (mut output_port, (main_port, fade_out_port)) in audio
            .output_ports()
            .zip(self.main_buffers.iter().zip(&self.fading_out_buffers))
        {
            let mut output_channels = output_port.channels()?.into_f32().unwrap(); // TODO: handle non-f32, check it matches

            for (output_channel, (main_channel, fade_out_channel)) in output_channels
                .iter_mut()
                .zip(main_port.iter().zip(fade_out_port))
            {
                cross_fader.apply_crossfade(main_channel, fade_out_channel, output_channel)
            }
        }

        let processed_frames = audio.frames_count().min(self.buffer_frame_count);
        cross_fader.advance(processed_frames);

        Ok(())
    }
}
