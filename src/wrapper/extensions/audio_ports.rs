use crate::wrapper::*;
use clack_extensions::audio_ports::*;

// TODO: handle rescan
pub struct PluginAudioPortsInfo {
    output_channels_count_per_port: Vec<u32>,
}

impl PluginAudioPortsInfo {
    pub fn new(plugin: &mut PluginInstance<WrapperHost>) -> Self {
        let mut info = Self {
            output_channels_count_per_port: Vec::new(),
        };
        info.update(plugin);
        info
    }

    pub fn output_channels_count_per_port(&self) -> &[u32] {
        &self.output_channels_count_per_port
    }

    pub fn update(&mut self, plugin: &mut PluginInstance<WrapperHost>) {
        let Some(audio_ports) = plugin.shared_handler().wrapped_plugin().audio_ports else {
            // Use default, single port stereo config
            self.output_channels_count_per_port.push(2);
            return;
        };

        let mut plugin = plugin.plugin_handle();
        self.output_channels_count_per_port.clear();

        let output_port_count = audio_ports.count(&mut plugin, false);

        let mut buf = AudioPortInfoBuffer::new();
        for i in 0..output_port_count {
            let Some(data) = audio_ports.get(&mut plugin, i, false, &mut buf) else {
                continue;
            };

            self.output_channels_count_per_port.push(data.channel_count);
        }
    }
}

impl<'a> HostAudioPortsImpl for WrapperHostMainThread<'a> {
    fn is_rescan_flag_supported(&self, flag: RescanType) -> bool {
        let Some(audio_ports) = self.shared.parent.audio_ports else {
            unreachable!()
        };

        audio_ports.is_rescan_flag_supported(&self.parent, flag)
    }

    fn rescan(&mut self, flag: RescanType) {
        let Some(audio_ports) = self.shared.parent.audio_ports else {
            unreachable!()
        };

        audio_ports.rescan(&mut self.parent, flag)
    }
}

impl<'a> PluginAudioPortsImpl for WrapperPluginMainThread<'a> {
    fn count(&mut self, is_input: bool) -> u32 {
        let host = self.plugin_instance.shared_handler();
        let Some(audio_ports) = host.wrapped_plugin().audio_ports else {
            return 0;
        };

        audio_ports.count(&mut self.plugin_handle(), is_input)
    }

    fn get(&mut self, index: u32, is_input: bool, writer: &mut AudioPortInfoWriter) {
        let host = self.plugin_instance.shared_handler();
        let Some(audio_ports) = host.wrapped_plugin().audio_ports else {
            return;
        };

        let mut buf = AudioPortInfoBuffer::new();

        if let Some(data) = audio_ports.get(&mut self.plugin_handle(), index, is_input, &mut buf) {
            writer.set(&data)
        }
    }
}
