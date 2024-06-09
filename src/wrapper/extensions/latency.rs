use crate::wrapper::extensions::OuterHostExtensions;
use crate::wrapper::requests::PluginMainThreadRequests;
use crate::wrapper::{WrapperHostMainThread, WrapperPluginMainThread};
use clack_extensions::latency::{HostLatencyImpl, PluginLatencyImpl};
use clack_plugin::prelude::HostMainThreadHandle;

impl<'a> PluginLatencyImpl for WrapperPluginMainThread<'a> {
    fn get(&mut self) -> u32 {
        let Some(latency) = self.wrapped_extensions().latency else {
            return 0;
        };

        latency.get(&mut self.plugin_handle())
    }
}

impl<'a> HostLatencyImpl for WrapperHostMainThread<'a> {
    fn changed(&mut self) {
        self.requests.latency_changed = true;
    }
}

impl PluginMainThreadRequests {
    pub fn process_latency_requests(
        &mut self,
        handle: &mut HostMainThreadHandle,
        extensions: &OuterHostExtensions,
    ) {
        if !self.latency_changed {
            return;
        }
        self.latency_changed = false;

        let Some(latency) = extensions.latency else {
            return;
        };
        latency.changed(handle);
    }
}
