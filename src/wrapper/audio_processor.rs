use crate::wrapper::*;
use clack_host::prelude::{PluginAudioConfiguration, ProcessStatus};
use clack_plugin::host::HostAudioThreadHandle;
use clack_plugin::plugin::{AudioConfiguration, PluginAudioProcessor, PluginError};
use clack_plugin::prelude::{Audio, Events, Process};

pub struct WrapperPluginAudioProcessor<'a> {
    host: HostAudioThreadHandle<'a>,
    shared: &'a WrapperPluginShared<'a>,
    pub(crate) audio_processor: clack_host::process::PluginAudioProcessor<WrapperHost>,
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
        // This is a really ugly hack, due to the fact that plugin instances are essentially 'static
        // for now. This is fixed in the plugin-instance-sublifetimes branch of clack but is blocked
        // on a borrow checker limitation bug:
        // https://internals.rust-lang.org/t/is-due-to-current-limitations-in-the-borrow-checker-overzealous/17818
        let host: HostAudioThreadHandle<'static> = unsafe { core::mem::transmute(host) };

        // TODO: why are the audio configs different...
        // TODO: unwrap
        let audio_processor = main_thread
            .plugin_instance
            .activate(
                |plugin, shared, _| WrapperHostAudioProcessor {
                    parent: host,
                    shared,
                    plugin,
                },
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
        let (audio_inputs, mut audio_outputs) = AudioPorts::from_plugin_audio_mut(&mut audio);

        self.audio_processor
            .ensure_processing_started()
            .map_err(|_| PluginError::Message("Not started"))?
            .process(
                &audio_inputs,
                &mut audio_outputs,
                events.input,
                events.output,
                process.steady_time.map(|i| i as i64).unwrap_or(-1), // FIXME: i64 consistency stuff
                None,
                process.transport,
            )
            .map_err(|_| PluginError::OperationFailed)
    }

    fn deactivate(self, main_thread: &mut WrapperPluginMainThread<'a>) {
        main_thread
            .plugin_instance
            .deactivate(self.audio_processor.into_stopped());
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
