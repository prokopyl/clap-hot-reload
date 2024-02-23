use crate::wrapper::*;
use clack_extensions::note_ports::*;

impl<'a> PluginNotePortsImpl for WrapperPluginMainThread<'a> {
    fn count(&mut self, is_input: bool) -> u32 {
        let host = self.plugin_instance.main_thread_host_data_mut();
        let Some(note_ports) = host.shared.wrapped_plugin().note_ports else {
            todo!()
        };

        note_ports.count(host.plugin.as_mut().unwrap(), is_input)
    }

    fn get(&mut self, is_input: bool, index: u32, writer: &mut NotePortInfoWriter) {
        let host = self.plugin_instance.main_thread_host_data_mut();
        let Some(note_ports) = host.shared.wrapped_plugin().note_ports else {
            todo!()
        };

        note_ports.get_to_writer(host.plugin.as_ref().unwrap(), is_input, index, writer);
    }
}
