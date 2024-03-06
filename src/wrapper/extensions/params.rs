use crate::wrapper::*;
use clack_extensions::params::*;
use clack_host::utils::Cookie;
use clack_plugin::utils::ClapId;
use std::ffi::CString;
use std::fmt::Write;
use std::mem::MaybeUninit;

struct CachedParamInfo {
    id: ClapId,
    flags: ParamInfoFlags,
    min_value: f64,
    max_value: f64,
    default_value: f64,
    name: String,
    module: String,
    cookie: Cookie,
}

impl CachedParamInfo {
    fn from_info(info: &ParamInfo) -> Self {
        Self {
            id: info.id,
            flags: info.flags,
            min_value: info.min_value,
            max_value: info.max_value,
            default_value: info.default_value,
            cookie: info.cookie,

            module: String::from_utf8_lossy(info.module).into_owned(),
            name: String::from_utf8_lossy(info.name).into_owned(),
        }
    }

    #[inline]
    fn as_info(&self) -> ParamInfo {
        ParamInfo {
            id: self.id,
            flags: self.flags,
            min_value: self.min_value,
            max_value: self.max_value,
            default_value: self.default_value,
            cookie: self.cookie,

            module: self.module.as_bytes(),
            name: self.name.as_bytes(),
        }
    }

    pub fn update(&mut self, info: &ParamInfo) -> ParamRescanFlags {
        let coarse_diff = self.as_info().diff_for_rescan(info);
        self.default_value = info.default_value;

        if !coarse_diff.is_empty() {
            self.flags = info.flags;
            self.min_value = info.min_value;
            self.max_value = info.max_value;
            self.cookie = info.cookie;

            self.module = String::from_utf8_lossy(info.module).into_owned();
            self.name = String::from_utf8_lossy(info.name).into_owned();
        }

        coarse_diff
    }
}

pub struct ParamInfoCache {
    params: Vec<CachedParamInfo>,
}

impl ParamInfoCache {
    pub fn new(instance: &mut PluginInstance<WrapperHost>) -> Self {
        let mut list = Self { params: vec![] };
        list.update(instance);
        list
    }

    pub fn update(&mut self, instance: &mut PluginInstance<WrapperHost>) -> ParamRescanFlags {
        let host = instance.main_thread_host_data_mut();

        let Some(params) = host.shared.wrapped_plugin().params else {
            return ParamRescanFlags::empty();
        };

        let plugin = host.plugin();

        let mut flags = ParamRescanFlags::empty();
        let mut buf = ParamInfoBuffer::new();

        let mut unseen_ids: Vec<_> = self.params.iter().map(|p| p.id).collect();
        let param_count = params.count(plugin);

        for i in 0..param_count {
            let Some(info) = params.get_info(plugin, i, &mut buf) else {
                continue;
            };

            let Some(matching_param) = self.params.iter_mut().find(|p| p.id == info.id) else {
                // New param!
                self.params.push(CachedParamInfo::from_info(&info));
                flags |= ParamRescanFlags::ALL;

                continue;
            };

            unseen_ids.retain(|i| *i != info.id);

            flags |= matching_param.as_info().diff_for_rescan(&info);
            matching_param.update(&info);
        }

        // Handle removed params
        if !unseen_ids.is_empty() {
            flags |= ParamRescanFlags::ALL;
            self.params.retain(|p| !unseen_ids.contains(&p.id))
        }

        flags
    }
}

// FIXME: Plugin params trait name + module paths are inconsistent
impl<'a> PluginMainThreadParams for WrapperPluginMainThread<'a> {
    fn count(&mut self) -> u32 {
        self.param_info_cache.params.len() as u32
    }

    fn get_info(&mut self, param_index: u32, writer: &mut ParamInfoWriter) {
        if let Some(param) = self.param_info_cache.params.get(param_index as usize) {
            writer.set(&param.as_info())
        }
    }

    fn get_value(&mut self, param_id: u32) -> Option<f64> {
        let host = self.plugin_instance.main_thread_host_data_mut();

        let Some(params) = host.shared.wrapped_plugin().params else {
            return None;
        };

        params.get_value(host.plugin(), param_id)
    }

    fn value_to_text(
        &mut self,
        param_id: u32,
        value: f64,
        writer: &mut ParamDisplayWriter,
    ) -> std::fmt::Result {
        let host = self.plugin_instance.main_thread_host_data_mut();

        let Some(params) = host.shared.wrapped_plugin().params else {
            todo!()
        };

        let mut buf = [MaybeUninit::zeroed(); 128];
        let str = params
            .value_to_text(host.plugin(), param_id, value, &mut buf) // TODO: make value_to_text return an error, not option
            .ok_or(core::fmt::Error)?;

        // FIXME: all of this is super ugly
        writer.write_str(core::str::from_utf8(str).unwrap())
    }

    fn text_to_value(&mut self, param_id: u32, text: &str) -> Option<f64> {
        let host = self.plugin_instance.main_thread_host_data_mut();

        let Some(params) = host.shared.wrapped_plugin().params else {
            return None;
        };

        // FIXME: alloc is unnecessary, it's already a C string pointer behind the scenes!
        let buf = CString::new(text).unwrap();
        params.text_to_value(host.plugin(), param_id, &buf)
    }

    fn flush(
        &mut self,
        input_parameter_changes: &InputEvents,
        output_parameter_changes: &mut OutputEvents,
    ) {
        let host = self.plugin_instance.main_thread_host_data_mut();

        let Some(params) = host.shared.wrapped_plugin().params else {
            return;
        };

        // FIXME: parameter name inconsistency between flush_active and this flush's params
        params.flush(
            host.plugin(),
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
        let host = self.current_audio_processor.audio_processor_host_data_mut();

        let Some(params) = host.shared.wrapped_plugin().params else {
            return;
        };

        // FIXME: parameter name inconsistency between flush_active and this flush's params
        params.flush_active(
            &mut host.plugin,
            input_parameter_changes,
            output_parameter_changes,
        );
    }
}
