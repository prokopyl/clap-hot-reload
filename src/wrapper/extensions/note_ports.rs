use crate::wrapper::*;
use clack_extensions::note_ports::*;

impl<'a> PluginNotePortsImpl for WrapperPluginMainThread<'a> {
    fn count(&mut self, is_input: bool) -> u32 {
        let host = self.plugin_instance.main_thread_host_data_mut();
        let Some(note_ports) = host.shared.wrapped_plugin().note_ports else {
            return 0;
        };

        note_ports.count(host.plugin.as_mut().unwrap(), is_input)
    }

    fn get(&mut self, index: u32, is_input: bool, writer: &mut NotePortInfoWriter) {
        let host = self.plugin_instance.main_thread_host_data_mut();
        let Some(note_ports) = host.shared.wrapped_plugin().note_ports else {
            return;
        };

        let mut buf = NotePortInfoBuffer::new();

        if let Some(data) = note_ports.get(host.plugin.as_mut().unwrap(), index, is_input, &mut buf)
        {
            writer.set(&data);
        }
    }
}
