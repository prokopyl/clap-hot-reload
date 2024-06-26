use crate::wrapper::WrapperPlugin;
use clack_extensions::audio_ports::PluginAudioPorts;
use clack_extensions::gui::{HostGui, PluginGui};
use clack_extensions::latency::{HostLatency, PluginLatency};
use clack_extensions::note_ports::PluginNotePorts;
use clack_extensions::params::{HostParams, PluginParams};
use clack_extensions::state::PluginState;
use clack_host::prelude::*;
use clack_plugin::prelude::*;

mod audio_ports;
mod gui;
mod latency;
mod note_ports;
mod params;
mod state;
mod timer;

pub use audio_ports::*;
pub use gui::*;
pub use latency::*;
pub use params::*;
pub use state::*;
pub use timer::*;

pub struct WrappedPluginExtensions {
    audio_ports: Option<PluginAudioPorts>,
    gui: Option<PluginGui>,
    latency: Option<PluginLatency>,
    note_ports: Option<PluginNotePorts>,
    params: Option<PluginParams>,
    state: Option<PluginState>,
}

impl WrappedPluginExtensions {
    pub fn new(handle: InitializingPluginHandle) -> Self {
        Self {
            audio_ports: handle.get_extension(),
            gui: handle.get_extension(),
            latency: handle.get_extension(),
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

// TODO: don't (necessarily?) report extensions based on plugin support
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

        builder.register::<PluginGui>();
        builder.register::<PluginLatency>();
    }
}

pub struct OuterHostExtensions {
    pub latency: Option<HostLatency>,
    pub params: Option<HostParams>,
    pub gui: Option<HostGui>,
}

impl OuterHostExtensions {
    pub fn new(host: &HostSharedHandle) -> Self {
        Self {
            latency: host.get_extension(),
            params: host.get_extension(),
            gui: host.get_extension(),
        }
    }
}
