use crate::wrapper::{WrapperHostMainThread, WrapperPluginMainThread};
use clack_extensions::audio_ports::*;

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
    fn count(&self, is_input: bool) -> u32 {
        self.plugin_instance
            .main_thread_host_data()
            .audio_ports_count(is_input)
    }

    fn get(&self, is_input: bool, index: u32, writer: &mut AudioPortInfoWriter) {
        todo!()
    }
}

impl<'a> WrapperHostMainThread<'a> {
    pub fn audio_ports_count(&self, is_input: bool) -> u32 {
        let Some(audio_ports) = self.shared.wrapped_plugin().audio_ports else {
            todo!()
        };

        audio_ports.count(self.plugin.as_ref().unwrap(), is_input)
    }
}
