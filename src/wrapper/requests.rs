use crate::wrapper::extensions::{OuterHostExtensions, PluginGuiRequests};
use clack_plugin::host::HostMainThreadHandle;
use clack_plugin::prelude::HostSharedHandle;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct PluginSharedRequests {
    callback_requested: AtomicBool,
    pub(super) gui: PluginGuiRequests,
}

impl PluginSharedRequests {
    pub fn new() -> Self {
        Self {
            callback_requested: AtomicBool::new(false),
            gui: PluginGuiRequests::new(),
        }
    }

    pub fn request_callback(&self) {
        self.callback_requested.store(true, Ordering::Relaxed)
    }

    pub fn process_requests(
        &self,
        parent_host: &HostSharedHandle,
        extensions: &OuterHostExtensions,
    ) {
        if let Ok(true) = self.callback_requested.compare_exchange(
            true,
            false,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            parent_host.request_callback()
        }

        self.gui.process_requests(parent_host, extensions);
    }
}

pub struct PluginMainThreadRequests {
    pub latency_changed: bool,
}

impl PluginMainThreadRequests {
    pub fn new() -> Self {
        Self {
            latency_changed: false,
        }
    }

    pub fn process_requests(
        &mut self,
        parent_host: &mut HostMainThreadHandle,
        extensions: &OuterHostExtensions,
    ) {
        self.process_latency_requests(parent_host, extensions)
    }
}
