use crate::wrapper::*;
use clack_extensions::params::implementation::*;
use std::ffi::CString;
use std::fmt::Write;
use std::mem::MaybeUninit;

// FIXME: Plugin params trait name + module paths are inconsistent
impl<'a> PluginMainThreadParams for WrapperPluginMainThread<'a> {
    fn count(&self) -> u32 {
        let host = self.plugin_instance.main_thread_host_data();

        let Some(params) = host.shared.wrapped_plugin().params else {
            todo!()
        };

        params.count(host.plugin.as_ref().unwrap())
    }

    fn get_info(&self, param_index: u32, info: &mut ParamInfoWriter) {
        let host = self.plugin_instance.main_thread_host_data();

        let Some(params) = host.shared.wrapped_plugin().params else {
            todo!()
        };

        params.get_info_to_writer(host.plugin.as_ref().unwrap(), param_index, info)
    }

    fn get_value(&self, param_id: u32) -> Option<f64> {
        let host = self.plugin_instance.main_thread_host_data();

        let Some(params) = host.shared.wrapped_plugin().params else {
            todo!()
        };

        params.get_value(host.plugin.as_ref().unwrap(), param_id)
    }

    fn value_to_text(
        &self,
        param_id: u32,
        value: f64,
        writer: &mut ParamDisplayWriter,
    ) -> std::fmt::Result {
        let host = self.plugin_instance.main_thread_host_data();

        let Some(params) = host.shared.wrapped_plugin().params else {
            todo!()
        };

        let mut buf = [MaybeUninit::zeroed(); 128];
        let str = params
            .value_to_text(host.plugin.as_ref().unwrap(), param_id, value, &mut buf)
            .ok_or(core::fmt::Error)?;

        // FIXME: all of this is super ugly
        writer.write_str(core::str::from_utf8(str).unwrap())
    }

    fn text_to_value(&self, param_id: u32, text: &str) -> Option<f64> {
        let host = self.plugin_instance.main_thread_host_data();

        let Some(params) = host.shared.wrapped_plugin().params else {
            todo!()
        };

        // FIXME: alloc is unnecessary, it's already a C string pointer behind the scenes!
        let buf = CString::new(text).unwrap();
        params.text_to_value(host.plugin.as_ref().unwrap(), param_id, &buf)
    }

    fn flush(
        &mut self,
        input_parameter_changes: &InputEvents,
        output_parameter_changes: &mut OutputEvents,
    ) {
        let host = self.plugin_instance.main_thread_host_data_mut();

        let Some(params) = host.shared.wrapped_plugin().params else {
            todo!()
        };

        // FIXME: parameter name inconsistency between flush_active and this flush's params
        params.flush(
            host.plugin.as_mut().unwrap(), // FIXME: unwrap
            input_parameter_changes,
            output_parameter_changes,
        );
    }
}

impl<'a> PluginAudioProcessorParams for WrapperPluginAudioProcessor<'a> {
    fn flush(
        &mut self,
        input_parameter_changes: &InputEvents,
        output_parameter_changes: &mut OutputEvents,
    ) {
        let host = self.audio_processor.audio_processor_host_data_mut();

        let Some(params) = host.shared.wrapped_plugin().params else {
            todo!()
        };

        // FIXME: parameter name inconsistency between flush_active and this flush's params
        params.flush_active(
            &mut host.plugin,
            input_parameter_changes,
            output_parameter_changes,
        );
    }
}
