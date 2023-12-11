use clack_host::prelude::*;
use clack_plugin::prelude::HostHandle;
use std::ffi::CStr;
use std::sync::OnceLock;

pub struct WrapperHost;

fn clone_host_info(parent_host_info: &clack_plugin::host::HostInfo) -> HostInfo {
    todo!()
}
impl WrapperHost {
    pub fn new_instance(
        host: HostHandle,
        bundle: PluginBundle,
        instantiated_plugin_id: &CStr,
    ) -> PluginInstance<Self> {
        let info = clone_host_info(&host.info());

        // This is a really ugly hack, due to the fact that plugin instances are essentially 'static
        // for now. This is fixed in the plugin-instance-sublifetimes branch of clack but is blocked
        // on a borrow checker limitation bug:
        // https://internals.rust-lang.org/t/is-due-to-current-limitations-in-the-borrow-checker-overzealous/17818
        let host: HostHandle<'static> = unsafe { core::mem::transmute(host) };

        // TODO: unwrap
        let instance = PluginInstance::<WrapperHost>::new(
            |_| WrapperHostShared::<'_>::new(host),
            |_s| (),
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
    type MainThread<'a> = ();
    type AudioProcessor<'a> = ();
}

pub struct WrapperHostShared<'a> {
    plugin: OnceLock<PluginSharedHandle<'a>>,
    parent: HostHandle<'a>,
}

impl<'a> WrapperHostShared<'a> {
    pub fn new(parent: HostHandle<'a>) -> Self {
        Self {
            plugin: OnceLock::new(),
            parent,
        }
    }
}

impl<'a> HostShared<'a> for WrapperHostShared<'a> {
    fn instantiated(&self, instance: PluginSharedHandle<'a>) {
        let _ = self.plugin.set(instance);
    }

    fn request_restart(&self) {
        self.parent.request_restart()
    }

    fn request_process(&self) {
        self.parent.request_process()
    }

    fn request_callback(&self) {
        self.parent.request_callback()
    }
}
