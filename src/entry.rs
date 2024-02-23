use crate::watcher::WatcherMaster;
use crate::wrapper::{WrapperHost, WrapperPlugin, WrapperPluginMainThread, WrapperPluginShared};
use clack_plugin::entry::prelude::*;
use std::ffi::{CStr, CString};

pub struct HotReloaderEntry {
    plugin_factory: Option<PluginFactoryWrapper<HotReloaderPluginFactory>>,
}

impl Entry for HotReloaderEntry {
    fn new(_bundle_path: &CStr) -> Result<Self, EntryLoadError> {
        unreachable!()
    }

    fn declare_factories<'a>(&'a self, builder: &mut EntryFactories<'a>) {
        if let Some(plugin_factory) = &self.plugin_factory {
            builder.register_factory(plugin_factory);
        }
    }
}

impl HotReloaderEntry {
    pub fn new(
        bundle_path: &CStr,
        inner_entry: &'static EntryDescriptor,
    ) -> Result<Self, EntryLoadError> {
        let watcher = WatcherMaster::new(inner_entry, bundle_path)?;

        if watcher.initial_bundle().get_plugin_factory().is_none() {
            return Ok(Self {
                plugin_factory: None,
            });
        }

        Ok(Self {
            plugin_factory: Some(PluginFactoryWrapper::new(HotReloaderPluginFactory::new(
                watcher,
            ))),
        })
    }
}

struct HotReloaderPluginFactory {
    watcher: WatcherMaster,
    descriptors: Vec<PluginDescriptor>,
}

impl HotReloaderPluginFactory {
    pub fn new(watcher: WatcherMaster) -> Self {
        let descriptors = if let Some(factory) = watcher.initial_bundle().get_plugin_factory() {
            factory
                .plugin_descriptors()
                .filter_map(clone_plugin_descriptor)
                .collect()
        } else {
            vec![]
        };

        Self {
            watcher,
            descriptors,
        }
    }
}

impl PluginFactory for HotReloaderPluginFactory {
    fn plugin_count(&self) -> u32 {
        self.descriptors.len() as u32
    }

    fn plugin_descriptor(&self, index: u32) -> Option<&PluginDescriptor> {
        self.descriptors.get(index as usize)
    }

    fn create_plugin<'a>(
        &'a self,
        host_info: HostInfo<'a>,
        plugin_id: &CStr,
    ) -> Option<PluginInstance<'a>> {
        let matching_descriptor = self.descriptors.iter().find(|d| d.id() == plugin_id)?;

        let plugin_id: CString = plugin_id.into();

        Some(PluginInstance::<'a>::new_with::<WrapperPlugin, _>(
            host_info,
            matching_descriptor,
            move |host| {
                let shared_host = host.shared();
                let watcher_handle = self
                    .watcher
                    .create_handle(move || shared_host.request_callback());

                let instance =
                    WrapperHost::new_instance(host, watcher_handle.current_bundle(), &plugin_id);

                Ok((
                    WrapperPluginShared::new(shared_host, instance.handle(), watcher_handle),
                    move |shared| WrapperPluginMainThread::new(host, shared, instance),
                ))
            },
        ))
    }
}

fn clone_plugin_descriptor(
    desc: clack_host::factory::PluginDescriptor,
) -> Option<PluginDescriptor> {
    PluginDescriptor::new(desc.id()?.to_str().ok()?, desc.name().to_str().ok()?)
        .with_vendor(desc.vendor()?.to_str().ok().unwrap_or(""))
        .with_url(desc.url()?.to_str().ok().unwrap_or(""))
        .with_manual_url(desc.manual_url()?.to_str().ok().unwrap_or(""))
        .with_support_url(desc.support_url()?.to_str().ok().unwrap_or(""))
        .with_version(desc.version()?.to_str().ok().unwrap_or(""))
        .with_description(desc.description()?.to_str().ok().unwrap_or(""))
        .with_features(desc.features())
}
