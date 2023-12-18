use crate::wrapper::{WrapperHost, WrapperPluginMainThread, WrapperPluginShared};
use clack_host::prelude::{
    InputAudioBuffers, OutputAudioBuffers, PluginAudioConfiguration, ProcessStatus,
};
use clack_plugin::host::HostAudioThreadHandle;
use clack_plugin::plugin::{AudioConfiguration, PluginAudioProcessor, PluginError};
use clack_plugin::prelude::{Audio, Events, Process};

pub struct WrapperPluginAudioProcessor<'a> {
    host: HostAudioThreadHandle<'a>,
    shared: &'a WrapperPluginShared<'a>,
    audio_processor: clack_host::process::PluginAudioProcessor<WrapperHost>,
}

impl<'a> PluginAudioProcessor<'a, WrapperPluginShared<'a>, WrapperPluginMainThread<'a>>
    for WrapperPluginAudioProcessor<'a>
{
    fn activate(
        host: HostAudioThreadHandle<'a>,
        main_thread: &mut WrapperPluginMainThread<'a>,
        shared: &'a WrapperPluginShared<'a>,
        audio_config: AudioConfiguration,
    ) -> Result<Self, PluginError> {
        // TODO: why are the audio configs different...
        // TODO: unwrap
        let audio_processor = main_thread
            .plugin_instance
            .activate(
                |_, _, _| (),
                PluginAudioConfiguration {
                    frames_count_range: audio_config.min_sample_count
                        ..=audio_config.max_sample_count,
                    sample_rate: audio_config.sample_rate,
                },
            )
            .unwrap();

        Ok(Self {
            host,
            shared,
            audio_processor: audio_processor.into(),
        })
    }

    fn process(
        &mut self,
        process: Process,
        mut audio: Audio,
        events: Events,
    ) -> Result<ProcessStatus, PluginError> {
        let frames_count = audio.frames_count();

        let (audio_inputs, audio_outputs) = audio.raw_buffers();
        let audio_inputs =
            unsafe { InputAudioBuffers::from_raw_buffers(audio_inputs, frames_count) };

        let mut audio_outputs =
            unsafe { OutputAudioBuffers::from_raw_buffers(audio_outputs, frames_count) };

        self.audio_processor
            .ensure_processing_started()
            .map_err(|_| PluginError::Message("Not started"))?
            .process(
                &audio_inputs,
                &mut audio_outputs,
                events.input,
                events.output,
                process.steady_time.map(|i| i as i64).unwrap_or(-1), // FIXME: i64 consistency stuff
                Some(frames_count as usize),                         // FIXME usize consistency
                process.transport,
            )
            .map_err(|_| PluginError::OperationFailed)
    }

    fn deactivate(self, main_thread: &mut WrapperPluginMainThread<'a>) {
        main_thread
            .plugin_instance
            .deactivate(self.audio_processor.stopped());
    }

    fn reset(&mut self) {
        // FIXME: there's no reset on host
        todo!()
    }

    fn start_processing(&mut self) -> Result<(), PluginError> {
        self.audio_processor.start_processing().unwrap(); // TODO: unwrap
        Ok(())
    }

    fn stop_processing(&mut self) {
        self.audio_processor.stop_processing().unwrap(); // TODO: unwrap
    }
}