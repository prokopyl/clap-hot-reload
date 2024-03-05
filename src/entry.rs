use crate::util::load_if_different_bundle;
use crate::watcher::WatcherMaster;
use crate::wrapper::{WrapperHost, WrapperPlugin, WrapperPluginMainThread, WrapperPluginShared};
use clack_host::bundle::PluginBundle;
use clack_plugin::entry::prelude::*;
use std::ffi::{CStr, CString};
use std::path::Path;

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
        // TODO: unwrap
        let bundle_path = bundle_path.to_str().unwrap();
        let initial_bundle = load_initial_bundle(inner_entry, bundle_path)?;

        if initial_bundle.get_plugin_factory().is_none() {
            return Ok(Self {
                plugin_factory: None,
            });
        }

        let watcher = WatcherMaster::new(initial_bundle.clone(), Path::new(bundle_path));

        let factory = match watcher {
            None => HotReloaderPluginFactory::new_non_reloading(initial_bundle),
            Some(w) => HotReloaderPluginFactory::new(w, &initial_bundle),
        };

        Ok(Self {
            plugin_factory: Some(PluginFactoryWrapper::new(factory)),
        })
    }
}

struct HotReloaderPluginFactory {
    watcher: Option<WatcherMaster>,
    static_bundle: Option<PluginBundle>,
    descriptors: Vec<PluginDescriptor>,
}

impl HotReloaderPluginFactory {
    pub fn new(watcher: WatcherMaster, initial_bundle: &PluginBundle) -> Self {
        let descriptors = if let Some(factory) = initial_bundle.get_plugin_factory() {
            factory
                .plugin_descriptors()
                .filter_map(clone_plugin_descriptor)
                .collect()
        } else {
            vec![]
        };

        Self {
            watcher: Some(watcher),
            static_bundle: None,
            descriptors,
        }
    }

    pub fn new_non_reloading(plugin_bundle: PluginBundle) -> Self {
        let descriptors = if let Some(factory) = plugin_bundle.get_plugin_factory() {
            factory
                .plugin_descriptors()
                .filter_map(clone_plugin_descriptor)
                .collect()
        } else {
            vec![]
        };

        Self {
            watcher: None,
            static_bundle: Some(plugin_bundle),
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

        Some(
            PluginInstance::<'a>::new_with_initializer::<WrapperPlugin, _>(
                host_info,
                matching_descriptor,
                move |host| {
                    let bundle_receiver = self.watcher.as_ref().map(|w| w.new_receiver());

                    let bundle = match &bundle_receiver {
                        None => self.static_bundle.as_ref().unwrap(), // PANIC: either static_bundle or watcher is always set.
                        Some(r) => r.current_bundle(),
                    };

                    let instance = WrapperHost::new_instance(host, bundle, &plugin_id);

                    Ok((
                        WrapperPluginShared::new(host.shared(), &instance),
                        move |shared| {
                            WrapperPluginMainThread::new(
                                host,
                                shared,
                                instance,
                                bundle_receiver,
                                plugin_id,
                            )
                        },
                    ))
                },
            ),
        )
    }
}

fn clone_plugin_descriptor(
    desc: clack_host::factory::PluginDescriptor,
) -> Option<PluginDescriptor> {
    Some(
        PluginDescriptor::new(desc.id()?.to_str().ok()?, desc.name()?.to_str().ok()?)
            .with_vendor(desc.vendor().and_then(|s| s.to_str().ok()).unwrap_or(""))
            .with_url(desc.url().and_then(|s| s.to_str().ok()).unwrap_or(""))
            .with_manual_url(
                desc.manual_url()
                    .and_then(|s| s.to_str().ok())
                    .unwrap_or(""),
            )
            .with_support_url(
                desc.support_url()
                    .and_then(|s| s.to_str().ok())
                    .unwrap_or(""),
            )
            .with_version(desc.version().and_then(|s| s.to_str().ok()).unwrap_or(""))
            .with_description(
                desc.description()
                    .and_then(|s| s.to_str().ok())
                    .unwrap_or(""),
            )
            .with_features(desc.features()),
    )
}

#[allow(unsafe_code)]
fn load_initial_bundle(
    initial_entry: &'static EntryDescriptor,
    self_path: &str,
) -> Result<PluginBundle, EntryLoadError> {
    let bundle = if let Ok(Some(different_bundle)) =
        load_if_different_bundle(initial_entry, Path::new(self_path))
    {
        println!("Different bundle loaded. AAAA");
        different_bundle
    } else {
        println!("Loading from the same bundle");
        // TODO: double utf8 check
        unsafe { PluginBundle::load_from_raw(initial_entry, self_path) }
            .map_err(|_| EntryLoadError)?
    };

    Ok(bundle)
}
