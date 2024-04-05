use crate::wrapper::*;
use clack_extensions::note_ports::*;

impl<'a> PluginNotePortsImpl for WrapperPluginMainThread<'a> {
    fn count(&mut self, is_input: bool) -> u32 {
        let Some(note_ports) = self.wrapped_extensions().note_ports else {
            return 0;
        };

        note_ports.count(&mut self.plugin_handle(), is_input)
    }

    fn get(&mut self, index: u32, is_input: bool, writer: &mut NotePortInfoWriter) {
        let Some(note_ports) = self.wrapped_extensions().note_ports else {
            return;
        };

        let mut buf = NotePortInfoBuffer::new();

        if let Some(data) = note_ports.get(&mut self.plugin_handle(), index, is_input, &mut buf) {
            writer.set(&data);
        }
    }
}
