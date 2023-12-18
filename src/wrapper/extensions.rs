use crate::wrapper::{WrapperHost, WrapperPlugin};
use clack_extensions::audio_ports::{HostAudioPorts, PluginAudioPorts};
use clack_host::prelude::*;
use clack_plugin::prelude::*;

mod audio_ports;

pub struct ParentHostExtensions<'a> {
    handle: HostHandle<'a>, // TODO: naming consistency with plugin side
    audio_ports: Option<&'a HostAudioPorts>,
}

impl<'a> ParentHostExtensions<'a> {
    pub fn new(handle: HostHandle<'a>) -> Self {
        Self {
            audio_ports: handle.extension(), // TODO: extension() naming consistency with plugin side
            handle,
        }
    }

    #[inline]
    pub fn handle(&self) -> &HostHandle<'a> {
        &self.handle
    }

    pub fn declare_to_plugin(&self, builder: &mut HostExtensions<WrapperHost>) {
        if self.audio_ports.is_some() {
            builder.register::<HostAudioPorts>();
        }
    }
}

pub struct WrappedPluginExtensions<'a> {
    handle: PluginSharedHandle<'a>,
    audio_ports: Option<&'a PluginAudioPorts>,
}

impl<'a> WrappedPluginExtensions<'a> {
    pub fn new(handle: PluginSharedHandle<'a>) -> Self {
        Self {
            audio_ports: handle.get_extension(),
            handle,
        }
    }

    #[inline]
    pub fn handle(&self) -> &PluginSharedHandle<'a> {
        &self.handle
    }

    pub fn declare_to_host(&self, builder: &mut PluginExtensions<WrapperPlugin>) {
        if self.audio_ports.is_some() {
            builder.register::<PluginAudioPorts>();
        }
    }
}
