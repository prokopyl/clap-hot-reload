use crate::watcher::BundleReceiver;
use clack_extensions::audio_ports::HostAudioPorts;
use clack_extensions::gui::HostGui;
use clack_extensions::latency::HostLatency;
use clack_extensions::params::{HostParams, ParamRescanFlags};
use clack_extensions::timer::PluginTimer;
use clack_host::prelude::*;
use clack_plugin::prelude::*;
use clack_plugin::prelude::{HostMainThreadHandle, HostSharedHandle};
use std::ffi::{CStr, CString};
use std::sync::OnceLock;

mod audio_processor;
mod channel;
mod extensions;
mod requests;

use audio_processor::*;
use channel::*;
use extensions::*;
use requests::*;

pub struct WrapperHost;

impl WrapperHost {
    pub fn new_instance(
        host: &HostMainThreadHandle,
        bundle: &PluginBundle,
        instantiated_plugin_id: &CStr,
    ) -> PluginInstance<Self> {
        let info = HostInfo::from_plugin(host);

        // TODO: unwrap
        let instance = PluginInstance::<WrapperHost>::new(
            |_| WrapperHostShared::new(),
            |s| WrapperHostMainThread::new(s),
            bundle,
            instantiated_plugin_id,
            &info,
        )
        .unwrap();

        instance
    }

    pub fn activate_instance(
        plugin_instance: &mut PluginInstance<WrapperHost>,
        audio_config: PluginAudioConfiguration,
    ) -> Result<StoppedPluginAudioProcessor<WrapperHost>, PluginInstanceError> {
        plugin_instance.activate(
            |shared, _| WrapperHostAudioProcessor { _shared: shared },
            audio_config,
        )
    }
}

impl HostHandlers for WrapperHost {
    type Shared<'a> = WrapperHostShared;
    type MainThread<'a> = WrapperHostMainThread<'a>;
    type AudioProcessor<'a> = WrapperHostAudioProcessor<'a>;

    fn declare_extensions(builder: &mut HostExtensions<Self>, _shared: &Self::Shared<'_>) {
        builder.register::<HostAudioPorts>();
        builder.register::<HostGui>();
        builder.register::<HostLatency>();
    }
}

pub struct WrapperHostShared {
    pub(crate) plugin: OnceLock<WrappedPluginExtensions>,
    requests: PluginSharedRequests,
}

impl WrapperHostShared {
    pub fn new() -> Self {
        Self {
            plugin: OnceLock::new(),
            requests: PluginSharedRequests::new(),
        }
    }

    pub fn wrapped_plugin(&self) -> &WrappedPluginExtensions {
        self.plugin.get().unwrap() // FIXME: unwrap
    }
}

impl<'a> SharedHandler<'a> for WrapperHostShared {
    fn initializing(&self, instance: InitializingPluginHandle<'a>) {
        let _ = self.plugin.set(WrappedPluginExtensions::new(instance));
    }

    fn request_restart(&self) {
        todo!()
    }

    fn request_process(&self) {
        todo!()
    }

    fn request_callback(&self) {
        self.requests.request_callback()
    }
}

pub struct WrapperHostMainThread<'a> {
    shared: &'a WrapperHostShared,
    plugin: Option<InitializedPluginHandle<'a>>,
    requests: PluginMainThreadRequests,
}

impl<'a> WrapperHostMainThread<'a> {
    pub fn new(shared: &'a WrapperHostShared) -> Self {
        Self {
            shared,
            plugin: None,
            requests: PluginMainThreadRequests::new(),
        }
    }

    pub fn process_requests(
        &mut self,
        parent_host: &mut HostMainThreadHandle,
        extensions: &OuterHostExtensions,
    ) {
        self.shared
            .requests
            .process_requests(parent_host, extensions);
        self.requests.process_requests(parent_host, extensions)
    }
}

impl<'a> MainThreadHandler<'a> for WrapperHostMainThread<'a> {
    fn initialized(&mut self, instance: InitializedPluginHandle<'a>) {
        self.plugin = Some(instance)
    }
}

pub struct WrapperHostAudioProcessor<'a> {
    _shared: &'a WrapperHostShared,
    // parent: HostAudioProcessorHandle<'a>,
}

impl<'a> AudioProcessorHandler<'a> for WrapperHostAudioProcessor<'a> {}

pub struct WrapperPlugin;

impl Plugin for WrapperPlugin {
    type AudioProcessor<'a> = WrapperPluginAudioProcessor<'a>;
    type Shared<'a> = WrapperPluginShared<'a>;
    type MainThread<'a> = WrapperPluginMainThread<'a>;

    fn declare_extensions(builder: &mut PluginExtensions<Self>, shared: Option<&Self::Shared<'_>>) {
        builder.register::<PluginTimer>();

        if let Some(shared) = shared {
            // TODO: find a way to report this even if the host queries extensions early
            shared.reported_extensions.declare_to_host(builder);
        }
    }
}

pub struct WrapperPluginShared<'a> {
    _host: HostSharedHandle<'a>,
    reported_extensions: ReportedExtensions,
    host_extensions: OuterHostExtensions,
}

impl<'a> WrapperPluginShared<'a> {
    pub fn new(host: HostSharedHandle<'a>, plugin_handle: &PluginInstance<WrapperHost>) -> Self {
        let reported_extensions =
            plugin_handle.access_shared_handler(|h| h.wrapped_plugin().report());

        Self {
            host_extensions: OuterHostExtensions::new(&host),
            _host: host,
            reported_extensions,
        }
    }
}

impl<'a> PluginShared<'a> for WrapperPluginShared<'a> {}

pub struct WrapperPluginMainThread<'a> {
    host: HostMainThreadHandle<'a>,
    shared: &'a WrapperPluginShared<'a>,
    plugin_instance: PluginInstance<WrapperHost>,
    bundle_receiver: Option<BundleReceiver>,
    pub timers: WrapperTimerHandler,
    audio_processor_channel: Option<MainThreadChannel>,
    plugin_id: CString,
    current_audio_config: Option<PluginAudioConfiguration>,
    audio_ports_info: PluginAudioPortsInfo,
    param_info_cache: ParamInfoCache,
    gui: WrapperGui,
}

impl<'a> PluginMainThread<'a, WrapperPluginShared<'a>> for WrapperPluginMainThread<'a> {
    fn on_main_thread(&mut self) {
        //self.timers.init(&mut self.host);

        /* if let Some(new_bundle) = self.shared.watcher_handle.check_new_bundle_available() {
            todo!()
        } */

        if let Some(channel) = &mut self.audio_processor_channel {
            channel.destroy_awaiting()
        }

        // TODO: make sure the plugin has requested one
        self.plugin_instance.call_on_main_thread_callback()
    }
}

impl<'a> WrapperPluginMainThread<'a> {
    pub fn new(
        mut host: HostMainThreadHandle<'a>,
        shared: &'a WrapperPluginShared<'a>,
        mut plugin_instance: PluginInstance<WrapperHost>,
        bundle_receiver: Option<BundleReceiver>,
        plugin_id: CString,
    ) -> Result<Self, PluginError> {
        // host.shared().request_callback(); // To finish configuring timers. TODO: Bitwig bug?
        let audio_ports = host.get_extension();
        Ok(Self {
            audio_ports_info: PluginAudioPortsInfo::new(
                &mut plugin_instance,
                &mut host,
                audio_ports,
            ),
            param_info_cache: ParamInfoCache::new(&mut plugin_instance),
            gui: WrapperGui::new(&host),

            host,
            shared,
            plugin_instance,
            bundle_receiver,
            plugin_id,

            timers: WrapperTimerHandler::new(),
            audio_processor_channel: None,
            current_audio_config: None,
        })
    }

    pub fn wrapped_extensions(&self) -> &WrappedPluginExtensions {
        self.plugin_instance
            .access_shared_handler(|h| h.wrapped_plugin())
    }

    pub fn plugin_handle(&mut self) -> PluginMainThreadHandle {
        self.plugin_instance.plugin_handle()
    }

    fn check_for_new_bundles(&mut self) {
        let Some(receiver) = self.bundle_receiver.as_mut() else {
            return;
        };

        if !receiver.receive_new_bundle() {
            return;
        }

        println!("Received new bundle!!");

        let mut new_instance =
            WrapperHost::new_instance(&self.host, receiver.current_bundle(), &self.plugin_id);

        if let Err(e) = transfer_state(&mut self.plugin_instance, &mut new_instance) {
            eprintln!("Could not transfer state to new hot-loaded instance: {e}");
        }

        let mut old_instance = core::mem::replace(&mut self.plugin_instance, new_instance);

        // TODO: handle the function crashing here and ending up with a partial swap?
        let required_rescan = self.param_info_cache.update(&mut self.plugin_instance);

        if let Some(host_params) = self.shared.host_extensions.params {
            // Always rescan text renderings, we can never really know if it changed or not
            host_params.rescan(&mut self.host, required_rescan | ParamRescanFlags::TEXT)
        }

        if let Err(e) =
            self.gui
                .transfer_gui(&mut old_instance, &mut self.plugin_instance, &mut self.host)
        {
            eprintln!("{e}"); // TODO: handle errors(?)
        }

        // If there's no channel, we aren't active or processing. No need to keep the old instance around.
        let Some(channel) = &mut self.audio_processor_channel else {
            drop(old_instance);
            return;
        };

        let needs_restart = required_rescan.requires_restart();

        if needs_restart {
            // Don't bother activating the new instance yet.
            channel.defer_destroy_if_active(old_instance);
        } else {
            let config = self.current_audio_config.unwrap(); // TODO: this should always be the case if channel exists (checked above).

            // TODO: unwrap
            let audio_processor =
                WrapperHost::activate_instance(&mut self.plugin_instance, config).unwrap();

            // TODO: handle errors
            let _ = channel.send_new_audio_processor(audio_processor, old_instance);
        }
    }

    fn deactivate_wrapped_instance(
        &mut self,
        audio_processor: StoppedPluginAudioProcessor<WrapperHost>,
    ) {
        // Figure out where it comes from
        // Can happen if we swapped but audio processor didn't (yet?)
        if audio_processor.matches(&self.plugin_instance) {
            self.plugin_instance.deactivate(audio_processor);
        } else if let Some(channel) = &mut self.audio_processor_channel {
            channel.deactivate_old_instance(audio_processor)
        } else {
            drop(audio_processor)
        }
    }

    pub fn process_requests(&mut self) {
        self.plugin_instance.access_handler_mut(|h| {
            h.process_requests(&mut self.host, &self.shared.host_extensions)
        })
    }
}
