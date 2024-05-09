use clack_plugin::prelude::HostSharedHandle;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct PluginRequests {
    callback_requested: AtomicBool,
}

impl PluginRequests {
    pub fn new() -> Self {
        Self {
            callback_requested: AtomicBool::new(false),
        }
    }

    pub fn request_callback(&self) {
        self.callback_requested.store(true, Ordering::Relaxed)
    }

    pub fn process_requests(&self, parent_host: &HostSharedHandle) {
        if let Ok(true) = self.callback_requested.compare_exchange(
            true,
            false,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            parent_host.request_callback()
        }
    }
}
