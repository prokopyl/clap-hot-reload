use crate::wrapper::*;
use clack_extensions::state::*;
use clack_host::stream::{InputStream, OutputStream};
use std::io::Cursor;

pub fn transfer_state(
    src: &mut PluginInstance<WrapperHost>,
    dst: &mut PluginInstance<WrapperHost>,
) -> Result<(), StateError> {
    let src_host = src.main_thread_host_data_mut();
    let Some(src_state) = src_host.shared.wrapped_plugin().state else {
        return Ok(());
    };

    let dst_host = dst.main_thread_host_data_mut();
    let Some(dst_state) = dst_host.shared.wrapped_plugin().state else {
        return Ok(());
    };

    let mut buf = Vec::with_capacity(4096);

    let mut output_stream = OutputStream::from_writer(&mut buf);
    src_state.save(src_host.plugin(), &mut output_stream)?;

    let mut cursor = Cursor::new(buf);
    let mut input_stream = InputStream::from_reader(&mut cursor);
    dst_state.load(dst_host.plugin(), &mut input_stream)?;

    Ok(())
}

impl<'a> PluginStateImpl for WrapperPluginMainThread<'a> {
    fn save(&mut self, output: &mut OutputStream) -> Result<(), PluginError> {
        let host = self.plugin_instance.main_thread_host_data_mut();

        let Some(state) = host.shared.wrapped_plugin().state else {
            todo!()
        };

        // FIXME: inconsistency: PluginError vs StateError
        state.save(host.plugin(), output).unwrap();
        Ok(())
    }

    fn load(&mut self, input: &mut InputStream) -> Result<(), PluginError> {
        let host = self.plugin_instance.main_thread_host_data_mut();

        let Some(state) = host.shared.wrapped_plugin().state else {
            todo!()
        };

        // FIXME: inconsistency: PluginError vs StateError
        state.load(host.plugin(), input).unwrap();
        Ok(())
    }
}
