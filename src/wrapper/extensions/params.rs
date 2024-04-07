use crate::wrapper::*;
use clack_extensions::params::*;
use clack_host::utils::Cookie;
use clack_plugin::utils::ClapId;
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
        let Some(params) = instance.shared_handler().wrapped_plugin().params else {
            return ParamRescanFlags::empty();
        };

        let mut plugin = instance.plugin_handle();

        let mut flags = ParamRescanFlags::empty();
        let mut buf = ParamInfoBuffer::new();

        let mut unseen_ids: Vec<_> = self.params.iter().map(|p| p.id).collect();
        let param_count = params.count(&mut plugin);

        for i in 0..param_count {
            let Some(info) = params.get_info(&mut plugin, i, &mut buf) else {
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
        let Some(params) = self.wrapped_extensions().params else {
            return None;
        };

        params.get_value(&mut self.plugin_handle(), param_id)
    }

    fn value_to_text(
        &mut self,
        param_id: u32,
        value: f64,
        writer: &mut ParamDisplayWriter,
    ) -> std::fmt::Result {
        let Some(params) = self.wrapped_extensions().params else {
            todo!()
        };

        let mut buf = [MaybeUninit::zeroed(); 128];
        let bytes = params.value_to_text(&mut self.plugin_handle(), param_id, value, &mut buf)?;
        let str = String::from_utf8_lossy(bytes);

        writer.write_str(&str)
    }

    fn text_to_value(&mut self, param_id: u32, text: &CStr) -> Option<f64> {
        let Some(params) = self.wrapped_extensions().params else {
            return None;
        };

        params.text_to_value(&mut self.plugin_handle(), param_id, text)
    }

    fn flush(
        &mut self,
        input_parameter_changes: &InputEvents,
        output_parameter_changes: &mut OutputEvents,
    ) {
        let Some(params) = self.wrapped_extensions().params else {
            return;
        };

        params.flush(
            &mut self.plugin_handle(),
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
        let Some(params) = self
            .current_audio_processor
            .shared_handler()
            .wrapped_plugin()
            .params
        else {
            return;
        };

        params.flush_active(
            &mut self.current_audio_processor.plugin_handle(),
            input_parameter_changes,
            output_parameter_changes,
        );
    }
}
