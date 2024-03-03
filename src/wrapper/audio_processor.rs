use crate::wrapper::*;
use clack_host::prelude::{PluginAudioConfiguration, ProcessStatus};
use clack_plugin::host::HostAudioThreadHandle;
use clack_plugin::plugin::{AudioConfiguration, PluginAudioProcessor, PluginError};
use clack_plugin::prelude::{Audio, Events, Process};

pub struct WrapperPluginAudioProcessor<'a> {
    host: HostAudioThreadHandle<'a>,
    shared: &'a WrapperPluginShared<'a>,
    pub(crate) current_audio_processor: clack_host::process::PluginAudioProcessor<WrapperHost>,
    channel: AudioProcessorChannel,
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
        main_thread.timers.init(&mut main_thread.host); // Do it now I guess... (TODO: fixme)

        // FIXME: Host should very much NOT be Copy or Clone
        let audio_processor =
            WrapperHost::activate_instance(&mut main_thread.plugin_instance, audio_config);

        // TODO: handle possible leftover channel
        let (main_thread_channel, audio_processor_channel) = MainThreadChannel::new_pair();
        main_thread.audio_processor_channel = Some(main_thread_channel);
        main_thread.current_audio_config = Some(audio_config);

        Ok(Self {
            host,
            shared,
            current_audio_processor: audio_processor.into(),
            channel: audio_processor_channel,
        })
    }

    fn process(
        &mut self,
        process: Process,
        mut audio: Audio,
        events: Events,
    ) -> Result<ProcessStatus, PluginError> {
        let (audio_inputs, mut audio_outputs) = AudioPorts::from_plugin_audio_mut(&mut audio);

        // Hot swap!
        // TODO: recover note events
        if let Some(new_processor) = self.channel.check_for_new_processor() {
            println!("Audio processor received new update. Hot-swapping.");
            let old_processor =
                core::mem::replace(&mut self.current_audio_processor, new_processor.into());

            self.channel.send_for_disposal(old_processor.into_stopped());
        }

        self.current_audio_processor
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
            .deactivate(self.current_audio_processor.into_stopped());

        // TODO: handle leftovers
        main_thread.audio_processor_channel = None;
    }

    fn reset(&mut self) {
        // FIXME: there's no reset on host
        todo!()
    }

    fn start_processing(&mut self) -> Result<(), PluginError> {
        if let Some(new_processor) = self.channel.move_to_latest_new_processor() {
            let new_processor = new_processor.start_processing().unwrap().into();
            let old_processor =
                core::mem::replace(&mut self.current_audio_processor, new_processor);

            self.channel.send_for_disposal(old_processor.into_stopped());
        }

        self.current_audio_processor.start_processing().unwrap(); // TODO: unwrap
        Ok(())
    }

    fn stop_processing(&mut self) {
        self.current_audio_processor.stop_processing().unwrap(); // TODO: unwrap

        if let Some(new_processor) = self.channel.move_to_latest_new_processor() {
            let new_processor = new_processor.into();
            let old_processor =
                core::mem::replace(&mut self.current_audio_processor, new_processor);

            self.channel.send_for_disposal(old_processor.into_stopped());
        }
    }
}
