use clack_extensions::audio_ports::{
    HostAudioPorts, HostAudioPortsImpl, PluginAudioPorts, RescanType,
};
use clack_host::prelude::*;
use clack_plugin::prelude::{HostHandle, HostMainThreadHandle};
use std::ffi::CStr;
use std::sync::OnceLock;

pub struct WrapperHost;

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
    type AudioProcessor<'a> = ();

    fn declare_extensions(builder: &mut HostExtensions<Self>, shared: &Self::Shared<'_>) {
        if shared.parent_audio_ports.is_some() {
            builder.register::<HostAudioPorts>();
        }
    }
}

pub(crate) struct WrappedPluginData<'a> {
    pub shared: PluginSharedHandle<'a>,
    // Extensions
    pub audio_ports: Option<&'a PluginAudioPorts>,
}

pub struct WrapperHostShared<'a> {
    pub(crate) plugin: OnceLock<WrappedPluginData<'a>>,
    parent: HostHandle<'a>,
    parent_audio_ports: Option<&'a HostAudioPorts>,
}

impl<'a> WrapperHostShared<'a> {
    pub fn new(parent: HostHandle<'a>) -> Self {
        Self {
            plugin: OnceLock::new(),
            parent,
            parent_audio_ports: parent.extension(),
        }
    }
}

impl<'a> HostShared<'a> for WrapperHostShared<'a> {
    fn instantiated(&self, instance: PluginSharedHandle<'a>) {
        let _ = self.plugin.set(WrappedPluginData {
            shared: instance,
            audio_ports: instance.get_extension(),
        });
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

impl<'a> HostAudioPortsImpl for WrapperHostMainThread<'a> {
    fn is_rescan_flag_supported(&self, flag: RescanType) -> bool {
        let Some(audio_ports) = self.shared.parent_audio_ports else {
            unreachable!()
        };

        audio_ports.is_rescan_flag_supported(&self.parent, flag)
    }

    fn rescan(&mut self, flag: RescanType) {
        let Some(audio_ports) = self.shared.parent_audio_ports else {
            unreachable!()
        };

        audio_ports.rescan(&mut self.parent, flag)
    }
}
