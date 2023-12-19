use crate::wrapper::*;
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
        let host = self.plugin_instance.main_thread_host_data();
        let Some(audio_ports) = host.shared.wrapped_plugin().audio_ports else {
            todo!()
        };

        audio_ports.count(host.plugin.as_ref().unwrap(), is_input)
    }

    fn get(&self, is_input: bool, index: u32, writer: &mut AudioPortInfoWriter) {
        let host = self.plugin_instance.main_thread_host_data();
        let Some(audio_ports) = host.shared.wrapped_plugin().audio_ports else {
            todo!()
        };

        audio_ports.get_to_writer(host.plugin.as_ref().unwrap(), is_input, index, writer);
    }
}
