use crate::wrapper::*;
use clack_extensions::state::*;
use clack_host::stream::{InputStream, OutputStream};
use std::io::Cursor;

pub fn transfer_state(
    src: &mut PluginInstance<WrapperHost>,
    dst: &mut PluginInstance<WrapperHost>,
) -> Result<(), StateError> {
    let Some(src_state) = src.shared_handler().wrapped_plugin().state else {
        return Ok(());
    };

    let Some(dst_state) = dst.shared_handler().wrapped_plugin().state else {
        return Ok(());
    };

    let mut buf = Vec::with_capacity(4096);

    let mut output_stream = OutputStream::from_writer(&mut buf);
    src_state.save(&mut src.plugin_handle(), &mut output_stream)?;

    let mut cursor = Cursor::new(buf);
    let mut input_stream = InputStream::from_reader(&mut cursor);
    dst_state.load(&mut src.plugin_handle(), &mut input_stream)?;

    Ok(())
}

impl<'a> PluginStateImpl for WrapperPluginMainThread<'a> {
    fn save(&mut self, output: &mut OutputStream) -> Result<(), PluginError> {
        let Some(state) = self.wrapped_extensions().state else {
            todo!()
        };

        // FIXME: inconsistency: PluginError vs StateError
        state.save(&mut self.plugin_handle(), output).unwrap();
        Ok(())
    }

    fn load(&mut self, input: &mut InputStream) -> Result<(), PluginError> {
        let Some(state) = self.wrapped_extensions().state else {
            todo!()
        };

        // FIXME: inconsistency: PluginError vs StateError
        state.load(&mut self.plugin_handle(), input).unwrap();
        Ok(())
    }
}
