use clack_extensions::timer::PluginTimer;
use clack_host::prelude::*;
use clack_plugin::prelude::*;
use clack_plugin::prelude::{HostHandle, HostMainThreadHandle};
use std::ffi::{CStr, CString};
use std::sync::OnceLock;

pub struct WrapperHost;

mod audio_processor;
use audio_processor::*;

mod extensions;
use crate::watcher::BundleReceiver;
use extensions::*;

mod channel;

use channel::*;

// TODO: better conversion
fn clone_host_info(parent_host_info: &clack_plugin::host::HostInfo) -> HostInfo {
    // TODO: all of the unwraps omg
    HostInfo::new(
        parent_host_info
            .name()
            .map(|s| s.to_str().unwrap())
            .unwrap_or_default(),
        parent_host_info
            .vendor()
            .map(|s| s.to_str().unwrap())
            .unwrap_or_default(),
        parent_host_info
            .url()
            .map(|s| s.to_str().unwrap())
            .unwrap_or_default(),
        parent_host_info
            .version()
            .map(|s| s.to_str().unwrap())
            .unwrap_or_default(),
    )
    .unwrap()
}

impl WrapperHost {
    pub fn new_instance(
        host: HostMainThreadHandle,
        bundle: &PluginBundle,
        instantiated_plugin_id: &CStr,
    ) -> PluginInstance<Self> {
        let info = clone_host_info(&host.shared().info());

        // This is a really ugly hack, due to the fact that plugin instances are essentially 'static
        // for now. This is fixed in the plugin-instance-sublifetimes branch of clack but is blocked
        // on a borrow checker limitation bug:
        // https://internals.rust-lang.org/t/is-due-to-current-limitations-in-the-borrow-checker-overzealous/17818
        let host: HostMainThreadHandle<'static> = unsafe { core::mem::transmute(host) };
        let shared = host.shared();

        // FIXME: HostMainThreadHandle should DEFINITELY NOT be copy
        // TODO: unwrap
        let instance = PluginInstance::<WrapperHost>::new(
            |_| WrapperHostShared::<'_>::new(shared),
            |s| WrapperHostMainThread::new(s, host),
            bundle,
            instantiated_plugin_id,
            &info,
        )
        .unwrap();

        instance
    }

    pub fn activate_instance(
        plugin_instance: &mut PluginInstance<WrapperHost>,
        audio_config: AudioConfiguration,
    ) -> StoppedPluginAudioProcessor<WrapperHost> {
        // TODO: why are the audio configs different...
        // TODO: unwrap
        let audio_processor = plugin_instance
            .activate(
                |plugin, shared, _| WrapperHostAudioProcessor { shared, plugin },
                PluginAudioConfiguration {
                    frames_count_range: audio_config.min_sample_count
                        ..=audio_config.max_sample_count,
                    sample_rate: audio_config.sample_rate,
                },
            )
            .unwrap();

        audio_processor
    }
}

impl Host for WrapperHost {
    type Shared<'a> = WrapperHostShared<'a>;
    type MainThread<'a> = WrapperHostMainThread<'a>;
    type AudioProcessor<'a> = WrapperHostAudioProcessor<'a>;

    fn declare_extensions(builder: &mut HostExtensions<Self>, shared: &Self::Shared<'_>) {
        shared.parent.declare_to_plugin(builder);
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
    // parent: HostAudioThreadHandle<'a>, // FIXME: audioProcessor vs audioThread
}

impl<'a> HostAudioProcessor<'a> for WrapperHostAudioProcessor<'a> {}

pub struct WrapperPlugin;

impl Plugin for WrapperPlugin {
    type AudioProcessor<'a> = WrapperPluginAudioProcessor<'a>;
    type Shared<'a> = WrapperPluginShared<'a>;
    type MainThread<'a> = WrapperPluginMainThread<'a>;

    fn declare_extensions(builder: &mut PluginExtensions<Self>, shared: &Self::Shared<'_>) {
        builder.register::<PluginTimer>();

        shared.reported_extensions.declare_to_host(builder);
    }
}

pub struct WrapperPluginShared<'a> {
    host: HostHandle<'a>,
    reported_extensions: ReportedExtensions,
}

impl<'a> WrapperPluginShared<'a> {
    pub fn new(host: HostHandle<'a>, plugin_handle: &PluginInstance<WrapperHost>) -> Self {
        let reported_extensions = plugin_handle.shared_host_data().wrapped_plugin().report();

        Self {
            host,
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
    current_audio_config: Option<AudioConfiguration>,
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

        self.plugin_instance.call_on_main_thread_callback()
    }
}

impl<'a> WrapperPluginMainThread<'a> {
    pub fn new(
        host: HostMainThreadHandle<'a>,
        shared: &'a WrapperPluginShared<'a>,
        plugin_instance: PluginInstance<WrapperHost>,
        bundle_receiver: Option<BundleReceiver>,
        plugin_id: CString,
    ) -> Result<Self, PluginError> {
        // host.shared().request_callback(); // To finish configuring timers. TODO: Bitwig bug?
        Ok(Self {
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

    fn check_for_new_bundles(&mut self) {
        let Some(receiver) = self.bundle_receiver.as_mut() else {
            return;
        };

        if !receiver.receive_new_bundle() {
            return;
        }

        println!("Received new bundle!!");

        let new_instance =
            WrapperHost::new_instance(self.host, receiver.current_bundle(), &self.plugin_id);
        let old_instance = core::mem::replace(&mut self.plugin_instance, new_instance);

        // This means the wrapper plugin is activated, and possibly processing audio.
        if let (Some(channel), Some(config)) =
            (&mut self.audio_processor_channel, self.current_audio_config)
        {
            let audio_processor = WrapperHost::activate_instance(&mut self.plugin_instance, config);

            // TODO: handle errors
            let _ = channel.send_new_audio_processor(audio_processor, old_instance);
        }
        // Just drop old_instance if it wasn't activated.
    }
}
