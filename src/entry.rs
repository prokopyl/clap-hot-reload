use crate::watcher::WatcherMaster;
use crate::wrapper::{WrapperHost, WrapperPlugin, WrapperPluginMainThread, WrapperPluginShared};
use clack_host::bundle::PluginBundle;
use clack_plugin::entry::prelude::*;
use libloading::Library;
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

        let watcher = WatcherMaster::new(initial_bundle, Path::new(bundle_path));

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

        Some(
            PluginInstance::<'a>::new_with_initializer::<WrapperPlugin, _>(
                host_info,
                matching_descriptor,
                move |host| {
                    let shared_host = host.shared();
                    let watcher_handle = self
                        .watcher
                        .create_handle(move || shared_host.request_callback());

                    let instance = WrapperHost::new_instance(
                        host,
                        watcher_handle.current_bundle(),
                        &plugin_id,
                    );

                    Ok((
                        WrapperPluginShared::new(shared_host, instance.handle(), watcher_handle),
                        move |shared| WrapperPluginMainThread::new(host, shared, instance),
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

const WRAPPED_ENTRY_SYMBOL_NAME: &CStr =
    unsafe { CStr::from_bytes_with_nul_unchecked(b"__clack_hotreload_wrapped_entry\0") };

fn do_load_bikeshed_me(
    initial_entry: &EntryDescriptor,
    self_path: &str,
) -> Result<Option<PluginBundle>, EntryLoadError> {
    let lib = unsafe { Library::new(self_path) }.map_err(|_| EntryLoadError)?;

    let symbol =
        unsafe { lib.get::<*mut EntryDescriptor>(WRAPPED_ENTRY_SYMBOL_NAME.to_bytes_with_nul()) }
            .map_err(|_| EntryLoadError)?;

    let loaded_entry: &*mut EntryDescriptor = &*symbol;
    if core::ptr::eq(initial_entry, *loaded_entry) {
        return Ok(None);
    }

    let bundle = unsafe {
        PluginBundle::load_from_symbol_in_library(self_path, lib, WRAPPED_ENTRY_SYMBOL_NAME)
    }
    .map_err(|_| EntryLoadError)?;

    Ok(Some(bundle))
}

fn load_initial_bundle(
    initial_entry: &'static EntryDescriptor,
    self_path: &str,
) -> Result<PluginBundle, EntryLoadError> {
    let bundle = if let Ok(Some(different_bundle)) = do_load_bikeshed_me(initial_entry, self_path) {
        println!("Different bundle loaded");
        different_bundle
    } else {
        println!("Loading from the same bundle");
        // TODO: double utf8 check
        unsafe { PluginBundle::load_from_raw(initial_entry, self_path) }
            .map_err(|_| EntryLoadError)?
    };

    Ok(bundle)
}
