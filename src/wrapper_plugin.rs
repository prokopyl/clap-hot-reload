use crate::wrapper_host::WrapperHost;
use clack_extensions::audio_ports::{AudioPortInfoWriter, PluginAudioPorts, PluginAudioPortsImpl};
use clack_host::plugin::PluginInstanceHandle;
use clack_host::prelude::*;
use clack_plugin::prelude::*;

pub struct WrapperPlugin;

impl Plugin for WrapperPlugin {
    type AudioProcessor<'a> = WrapperPluginAudioProcessor<'a>;
    type Shared<'a> = WrapperPluginShared<'a>;
    type MainThread<'a> = WrapperPluginMainThread<'a>;

    fn get_descriptor() -> Box<dyn PluginDescriptor> {
        unreachable!()
    }

    fn declare_extensions(builder: &mut PluginExtensions<Self>, shared: &Self::Shared<'_>) {
        // TODO: this locks a lot
        shared
            .plugin_handle
            .use_shared_host_data(|shared| {
                let plugin_data = shared.plugin.get().unwrap();
                if plugin_data.audio_ports.is_some() {
                    builder.register::<PluginAudioPorts>();
                }
            })
            .unwrap();
    }
}

pub struct WrapperPluginShared<'a> {
    host: HostHandle<'a>,
    plugin_handle: PluginInstanceHandle<WrapperHost>,
}

impl<'a> WrapperPluginShared<'a> {
    pub fn new(host: HostHandle<'a>, plugin_handle: PluginInstanceHandle<WrapperHost>) -> Self {
        Self {
            host,
            plugin_handle,
        }
    }
}

impl<'a> PluginShared<'a> for WrapperPluginShared<'a> {
    fn new(_host: HostHandle<'a>) -> Result<Self, PluginError> {
        unreachable!()
    }
}

pub struct WrapperPluginMainThread<'a> {
    host: HostMainThreadHandle<'a>,
    shared: &'a WrapperPluginShared<'a>,
    plugin_instance: PluginInstance<WrapperHost>,
}

impl<'a> PluginMainThread<'a, WrapperPluginShared<'a>> for WrapperPluginMainThread<'a> {
    fn new(
        _host: HostMainThreadHandle<'a>,
        _shared: &'a WrapperPluginShared<'a>,
    ) -> Result<Self, PluginError> {
        unreachable!()
    }

    fn on_main_thread(&mut self) {
        self.plugin_instance.call_on_main_thread_callback()
    }
}

impl<'a> WrapperPluginMainThread<'a> {
    pub fn new(
        host: HostMainThreadHandle<'a>,
        shared: &'a WrapperPluginShared<'a>,
        plugin_instance: PluginInstance<WrapperHost>,
    ) -> Result<Self, PluginError> {
        Ok(Self {
            host,
            shared,
            plugin_instance,
        })
    }
}

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

impl<'a> PluginAudioPortsImpl for WrapperPluginMainThread<'a> {
    fn count(&self, is_input: bool) -> u32 {
        self.shared
            .plugin_handle
            .use_shared_host_data(|shared| {
                // TODO: unwraps
                let plugin_data = shared.plugin.get().unwrap();
                plugin_data
                    .audio_ports
                    .unwrap()
                    .count(&self.plugin_instance.main_thread_plugin_data(), is_input)
            })
            .unwrap()
    }

    fn get(&self, is_input: bool, index: u32, writer: &mut AudioPortInfoWriter) {
        todo!()
    }
}
