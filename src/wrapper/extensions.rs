use crate::wrapper::{WrapperHost, WrapperPlugin};
use clack_extensions::audio_ports::{HostAudioPorts, PluginAudioPorts};
use clack_extensions::note_ports::PluginNotePorts;
use clack_extensions::params::PluginParams;
use clack_extensions::state::PluginState;
use clack_host::prelude::*;
use clack_plugin::prelude::*;

mod audio_ports;
pub use audio_ports::*;

mod note_ports;
mod params;
pub use params::*;
mod state;
pub use state::*;
mod timer;
pub use timer::*;

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
    audio_ports: Option<&'a PluginAudioPorts>,
    note_ports: Option<&'a PluginNotePorts>,
    params: Option<&'a PluginParams>,
    state: Option<&'a PluginState>,
}

impl<'a> WrappedPluginExtensions<'a> {
    pub fn new(handle: PluginSharedHandle<'a>) -> Self {
        Self {
            audio_ports: handle.get_extension(),
            note_ports: handle.get_extension(),
            params: handle.get_extension(),
            state: handle.get_extension(),
        }
    }

    pub fn report(&self) -> ReportedExtensions {
        ReportedExtensions {
            audio_ports: self.audio_ports.is_some(),
            note_ports: self.note_ports.is_some(),
            params: self.params.is_some(),
            state: self.state.is_some(),
        }
    }
}

#[derive(Clone)]
pub struct ReportedExtensions {
    audio_ports: bool,
    note_ports: bool,
    params: bool,
    state: bool,
}

impl ReportedExtensions {
    pub fn declare_to_host(&self, builder: &mut PluginExtensions<WrapperPlugin>) {
        if self.audio_ports {
            builder.register::<PluginAudioPorts>();
        }

        if self.note_ports {
            builder.register::<PluginNotePorts>();
        }

        if self.params {
            builder.register::<PluginParams>();
        }

        if self.state {
            builder.register::<PluginState>();
        }
    }
}
