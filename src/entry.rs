use crate::wrapper::{WrapperHost, WrapperPlugin, WrapperPluginMainThread, WrapperPluginShared};
use clack_host::prelude::PluginBundle;
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
        let bundle_path = bundle_path.to_str().map_err(|_| EntryLoadError)?;
        let bundle = unsafe { PluginBundle::load_from_raw(inner_entry, bundle_path) }
            .map_err(|_| EntryLoadError)?;

        match bundle.get_plugin_factory() {
            None => Ok(Self {
                plugin_factory: None,
            }),
            Some(_) => Ok(Self {
                plugin_factory: Some(PluginFactoryWrapper::new(HotReloaderPluginFactory::new(
                    bundle,
                ))),
            }),
        }
    }
}

struct HotReloaderPluginFactory {
    inner_bundle: PluginBundle,
    descriptors: Vec<PluginDescriptorWrapper>,
}

impl HotReloaderPluginFactory {
    pub fn new(inner_bundle: PluginBundle) -> Self {
        let descriptors = if let Some(factory) = inner_bundle.get_plugin_factory() {
            factory
                .plugin_descriptors()
                .map(|d| PluginDescriptorWrapper::new(Box::new(ClonedPluginDescriptor::new(&d))))
                .collect()
        } else {
            vec![]
        };

        Self {
            inner_bundle,
            descriptors,
        }
    }
}

impl PluginFactory for HotReloaderPluginFactory {
    fn plugin_count(&self) -> u32 {
        self.descriptors.len() as u32
    }

    fn plugin_descriptor(&self, index: u32) -> Option<&PluginDescriptorWrapper> {
        self.descriptors.get(index as usize)
    }

    fn instantiate_plugin<'a>(
        &'a self,
        host_info: HostInfo<'a>,
        plugin_id: &CStr,
    ) -> Option<PluginInstance<'a>> {
        let matching_descriptor = self.descriptors.iter().find(|d| d.id() == plugin_id)?;

        let plugin_id: CString = plugin_id.into();
        let plugin_bundle = self.inner_bundle.clone();

        Some(PluginInstance::<'a>::new_with::<WrapperPlugin, _>(
            host_info,
            matching_descriptor,
            move |host| {
                let shared_host = host.shared();
                let instance = WrapperHost::new_instance(host, plugin_bundle, &plugin_id);
                Ok((
                    WrapperPluginShared::new(shared_host, instance.handle()),
                    move |shared| WrapperPluginMainThread::new(host, shared, instance),
                ))
            },
        ))
    }
}

// TODO: bikeshed
struct ClonedPluginDescriptor {
    id: CString,
    name: CString,
    vendor: Option<CString>,
    url: Option<CString>,
    manual_url: Option<CString>,
    support_url: Option<CString>,
    version: Option<CString>,
    description: Option<CString>,
    features: Vec<CString>,
}

impl ClonedPluginDescriptor {
    pub fn new(wrapped: &clack_host::factory::PluginDescriptor) -> Self {
        Self {
            id: wrapped.id().unwrap_or_default().into(),
            name: wrapped.name().unwrap_or_default().into(),
            vendor: wrapped.vendor().map(|s| s.into()),
            url: wrapped.url().map(|s| s.into()),
            manual_url: wrapped.manual_url().map(|s| s.into()),
            support_url: wrapped.support_url().map(|s| s.into()),
            version: wrapped.version().map(|s| s.into()),
            description: wrapped.description().map(|s| s.into()),
            features: wrapped.features().map(|s| s.into()).collect(),
        }
    }
}

impl clack_plugin::prelude::PluginDescriptor for ClonedPluginDescriptor {
    fn id(&self) -> &CStr {
        &self.id
    }

    fn name(&self) -> &CStr {
        &self.name
    }

    fn vendor(&self) -> Option<&CStr> {
        self.vendor.as_deref()
    }

    fn url(&self) -> Option<&CStr> {
        self.url.as_deref()
    }

    fn manual_url(&self) -> Option<&CStr> {
        self.manual_url.as_deref()
    }

    fn support_url(&self) -> Option<&CStr> {
        self.support_url.as_deref()
    }

    fn version(&self) -> Option<&CStr> {
        self.version.as_deref()
    }

    fn description(&self) -> Option<&CStr> {
        self.description.as_deref()
    }

    fn feature_at(&self, index: usize) -> Option<&CStr> {
        self.features.get(index).map(|s| s.as_c_str())
    }

    fn features_count(&self) -> usize {
        self.features.len()
    }
}
