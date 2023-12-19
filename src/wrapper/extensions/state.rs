use crate::wrapper::*;
use clack_extensions::state::*;
use clack_host::stream::{InputStream, OutputStream};

impl<'a> PluginStateImpl for WrapperPluginMainThread<'a> {
    fn save(&mut self, output: &mut OutputStream) -> Result<(), PluginError> {
        let host = self.plugin_instance.main_thread_host_data_mut();

        let Some(state) = host.shared.wrapped_plugin().state else {
            todo!()
        };

        // FIXME: inconsistency: PluginError vs StateError
        state.save(host.plugin.as_mut().unwrap(), output).unwrap();
        Ok(())
    }

    fn load(&mut self, input: &mut InputStream) -> Result<(), PluginError> {
        let host = self.plugin_instance.main_thread_host_data_mut();

        let Some(state) = host.shared.wrapped_plugin().state else {
            todo!()
        };

        // FIXME: inconsistency: PluginError vs StateError
        state.load(host.plugin.as_mut().unwrap(), input).unwrap();
        Ok(())
    }
}
