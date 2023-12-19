use clack_host::plugin::PluginInstanceHandle;
use clack_host::prelude::*;
use clack_plugin::prelude::*;
use clack_plugin::prelude::{HostHandle, HostMainThreadHandle};
use std::ffi::CStr;
use std::sync::OnceLock;

pub struct WrapperHost;

mod audio_processor;
use audio_processor::*;

mod extensions;
use extensions::*;

fn clone_host_info(parent_host_info: &clack_plugin::host::HostInfo) -> HostInfo {
    todo!()
}

impl WrapperHost {
    pub fn new_instance(
        host: HostMainThreadHandle,
        bundle: PluginBundle,
        instantiated_plugin_id: &CStr,
    ) -> PluginInstance<Self> {
        let info = clone_host_info(&host.shared().info());

        // This is a really ugly hack, due to the fact that plugin instances are essentially 'static
        // for now. This is fixed in the plugin-instance-sublifetimes branch of clack but is blocked
        // on a borrow checker limitation bug:
        // https://internals.rust-lang.org/t/is-due-to-current-limitations-in-the-borrow-checker-overzealous/17818
        let host: HostMainThreadHandle<'static> = unsafe { core::mem::transmute(host) };
        let shared = host.shared();

        // TODO: unwrap
        let instance = PluginInstance::<WrapperHost>::new(
            |_| WrapperHostShared::<'_>::new(shared),
            |s| WrapperHostMainThread::new(s, host),
            &bundle,
            instantiated_plugin_id,
            &info,
        )
        .unwrap();

        instance
    }
}

impl Host for WrapperHost {
    type Shared<'a> = WrapperHostShared<'a>;
    type MainThread<'a> = WrapperHostMainThread<'a>;
    type AudioProcessor<'a> = WrapperHostAudioProcessor<'a>;

    fn declare_extensions(builder: &mut HostExtensions<Self>, shared: &Self::Shared<'_>) {
        shared.parent.declare_to_plugin(builder)
    }
}

pub struct WrapperHostShared<'a> {
    pub(crate) plugin: OnceLock<WrappedPluginExtensions<'a>>,
    parent: ParentHostExtensions<'a>,
}

impl<'a> WrapperHostShared<'a> {
    pub fn new(parent: HostHandle<'a>) -> Self {
        Self {
            plugin: OnceLock::new(),
            parent: ParentHostExtensions::new(parent),
        }
    }

    pub fn wrapped_plugin(&self) -> &WrappedPluginExtensions<'a> {
        &self.plugin.get().unwrap() // FIXME: unwrap
    }
}

impl<'a> HostShared<'a> for WrapperHostShared<'a> {
    fn instantiated(&self, instance: PluginSharedHandle<'a>) {
        let _ = self.plugin.set(WrappedPluginExtensions::new(instance));
    }

    fn request_restart(&self) {
        self.parent.handle().request_restart()
    }

    fn request_process(&self) {
        self.parent.handle().request_process()
    }

    fn request_callback(&self) {
        self.parent.handle().request_callback()
    }
}

pub struct WrapperHostMainThread<'a> {
    shared: &'a WrapperHostShared<'a>,
    plugin: Option<PluginMainThreadHandle<'a>>,
    parent: HostMainThreadHandle<'a>,
}

impl<'a> WrapperHostMainThread<'a> {
    pub fn new(shared: &'a WrapperHostShared<'a>, parent: HostMainThreadHandle<'a>) -> Self {
        Self {
            shared,
            parent,
            plugin: None,
        }
    }
}

impl<'a> HostMainThread<'a> for WrapperHostMainThread<'a> {
    fn instantiated(&mut self, instance: PluginMainThreadHandle<'a>) {
        self.plugin = Some(instance)
    }
}

pub struct WrapperHostAudioProcessor<'a> {
    shared: &'a WrapperHostShared<'a>,
    plugin: PluginAudioProcessorHandle<'a>,
    parent: HostAudioThreadHandle<'a>, // FIXME: audioProcessor vs audioThread
}

impl<'a> HostAudioProcessor<'a> for WrapperHostAudioProcessor<'a> {}

pub struct WrapperPlugin;

impl Plugin for WrapperPlugin {
    type AudioProcessor<'a> = WrapperPluginAudioProcessor<'a>;
    type Shared<'a> = WrapperPluginShared<'a>;
    type MainThread<'a> = WrapperPluginMainThread<'a>;

    fn get_descriptor() -> Box<dyn PluginDescriptor> {
        unreachable!()
    }

    fn declare_extensions(builder: &mut PluginExtensions<Self>, shared: &Self::Shared<'_>) {
        // TODO: this locks a lot
        shared
            .plugin_handle
            // FIXME: unwrap
            .use_shared_host_data(|shared| shared.plugin.get().unwrap().declare_to_host(builder))
            .unwrap();
    }
}

pub struct WrapperPluginShared<'a> {
    host: HostHandle<'a>,
    plugin_handle: PluginInstanceHandle<WrapperHost>,
}

impl<'a> WrapperPluginShared<'a> {
    pub fn new(host: HostHandle<'a>, plugin_handle: PluginInstanceHandle<WrapperHost>) -> Self {
        Self {
            host,
            plugin_handle,
        }
    }
}

impl<'a> PluginShared<'a> for WrapperPluginShared<'a> {
    fn new(_host: HostHandle<'a>) -> Result<Self, PluginError> {
        unreachable!()
    }
}

pub struct WrapperPluginMainThread<'a> {
    host: HostMainThreadHandle<'a>,
    shared: &'a WrapperPluginShared<'a>,
    plugin_instance: PluginInstance<WrapperHost>,
}

impl<'a> PluginMainThread<'a, WrapperPluginShared<'a>> for WrapperPluginMainThread<'a> {
    fn new(
        _host: HostMainThreadHandle<'a>,
        _shared: &'a WrapperPluginShared<'a>,
    ) -> Result<Self, PluginError> {
        unreachable!()
    }

    fn on_main_thread(&mut self) {
        self.plugin_instance.call_on_main_thread_callback()
    }
}

impl<'a> WrapperPluginMainThread<'a> {
    pub fn new(
        host: HostMainThreadHandle<'a>,
        shared: &'a WrapperPluginShared<'a>,
        plugin_instance: PluginInstance<WrapperHost>,
    ) -> Result<Self, PluginError> {
        Ok(Self {
            host,
            shared,
            plugin_instance,
        })
    }
}
