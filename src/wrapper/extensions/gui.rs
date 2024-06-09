#![allow(unsafe_code)] // Needed for raw window handles

use crate::wrapper::extensions::OuterHostExtensions;
use crate::wrapper::{WrapperHost, WrapperHostShared, WrapperPluginMainThread};
use clack_extensions::gui::{
    GuiConfiguration, GuiResizeHints, GuiSize, HostGui, HostGuiImpl, PluginGui, PluginGuiImpl,
    Window,
};
use clack_host::host::HostError;
use clack_host::prelude::PluginInstance;
use clack_plugin::plugin::PluginError;
use clack_plugin::prelude::{HostMainThreadHandle, HostSharedHandle};
use crossbeam_utils::atomic::AtomicCell;
use std::ffi::CString;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, Ordering};

enum Status {
    Destroyed,
    Created(GuiConfiguration<'static>),
}

pub struct WrapperGui {
    host_gui: Option<HostGui>,
    status: Status,

    size: Option<GuiSize>,
    scale: Option<f64>,
    shown: bool,
    title: Option<CString>,
    parent: Option<Window<'static>>,
    transient: Option<Window<'static>>,
}

impl WrapperGui {
    pub fn new(handle: &HostSharedHandle) -> Self {
        Self {
            host_gui: handle.get_extension(),
            status: Status::Destroyed,
            size: None,
            scale: None,
            shown: false,
            title: None,
            parent: None,
            transient: None,
        }
    }

    fn reset(&mut self) {
        self.status = Status::Destroyed;
        self.scale = None;
        self.size = None;
        self.shown = false;
        self.title = None;
        self.parent = None;
        self.transient = None;
    }

    pub fn transfer_gui(
        &self,
        old_instance: &mut PluginInstance<WrapperHost>,
        new_instance: &mut PluginInstance<WrapperHost>,
        host: &mut HostMainThreadHandle,
    ) -> Result<(), PluginError> {
        // TODO: this all assumes the host is fine in its sequencing
        if let Some(gui) = old_instance.access_shared_handler(|s| s.wrapped_plugin().gui) {
            let old_instance_handle = &mut old_instance.plugin_handle();
            if self.shown {
                let _ = gui.hide(old_instance_handle);
            }

            if let Status::Created(_) = &self.status {
                gui.destroy(old_instance_handle);
            }
        }

        // No need to open a new GUI if not supported
        let Some(gui) = new_instance.access_shared_handler(|s| s.wrapped_plugin().gui) else {
            return Ok(());
        };
        let plugin_handle = &mut new_instance.plugin_handle();

        // No need to open a new GUI if it wasn't open in the first place
        let Status::Created(config) = &self.status else {
            return Ok(());
        };

        // TODO: check config is supported, kill GUI otherwise

        gui.create(plugin_handle, *config)?;

        if config.is_floating {
            if let Some(transient) = self.transient {
                unsafe { gui.set_transient(plugin_handle, transient)? };
            }

            if let Some(title) = &self.title {
                gui.suggest_title(plugin_handle, title);
            }
        } else {
            if let Some(scale) = self.scale {
                gui.set_scale(plugin_handle, scale)?; // TODO: errors
            }

            if gui.can_resize(plugin_handle) {
                if let Some(size) = self.size {
                    gui.set_size(plugin_handle, size)?; // TODO: errors
                }
            } else if let Some(host_gui) = self.host_gui {
                if let Some(size) = gui.get_size(plugin_handle) {
                    // TODO: prevent spurious resizes if UI is of the same size
                    let _ = host_gui.request_resize(host, size.width, size.height);
                }
            }

            if let Some(parent) = self.parent {
                unsafe { gui.set_parent(plugin_handle, parent)? }; // TODO: errors
            }
        }

        if self.shown {
            gui.show(plugin_handle)?; // TODO: errors
        }

        Ok(())
    }
}

impl<'a> WrapperPluginMainThread<'a> {
    fn plugin_instance_gui(&self) -> Option<PluginGui> {
        self.plugin_instance
            .access_shared_handler(|h| h.wrapped_plugin().gui)
    }
}

// TODO: check what can be hot-reloaded
impl<'a> PluginGuiImpl for WrapperPluginMainThread<'a> {
    fn is_api_supported(&mut self, configuration: GuiConfiguration) -> bool {
        let Some(gui) = self.plugin_instance_gui() else {
            return false;
        };

        gui.is_api_supported(&mut self.plugin_handle(), configuration)
    }

    fn get_preferred_api(&mut self) -> Option<GuiConfiguration> {
        let gui = self.plugin_instance_gui()?;

        let config = gui.get_preferred_api(&mut self.plugin_handle())?;

        // TODO: add helper to standard api config
        Some(GuiConfiguration {
            api_type: config.api_type.to_standard_api()?,
            is_floating: config.is_floating,
        })
    }

    fn create(&mut self, configuration: GuiConfiguration) -> Result<(), PluginError> {
        let Some(gui) = self.plugin_instance_gui() else {
            return Err(PluginError::Message("Plugin does not support GUI"));
        };

        let configuration = GuiConfiguration {
            api_type: configuration.api_type.to_standard_api().unwrap(),
            is_floating: configuration.is_floating,
        };

        gui.create(&mut self.plugin_handle(), configuration)?;
        self.gui.status = Status::Created(configuration);
        Ok(())
    }

    fn destroy(&mut self) {
        let Some(gui) = self.plugin_instance_gui() else {
            return;
        };

        gui.destroy(&mut self.plugin_handle());

        self.gui.reset();
    }

    fn set_scale(&mut self, scale: f64) -> Result<(), PluginError> {
        let Some(gui) = self.plugin_instance_gui() else {
            return Err(PluginError::Message("Plugin does not support GUI"));
        };

        gui.set_scale(&mut self.plugin_handle(), scale)?;
        self.gui.scale = Some(scale);

        Ok(())
    }

    fn get_size(&mut self) -> Option<GuiSize> {
        let gui = self.plugin_instance_gui()?;

        gui.get_size(&mut self.plugin_handle())
    }

    fn can_resize(&mut self) -> bool {
        let Some(gui) = self.plugin_instance_gui() else {
            return false;
        };

        gui.can_resize(&mut self.plugin_handle())
    }

    fn get_resize_hints(&mut self) -> Option<GuiResizeHints> {
        let gui = self.plugin_instance_gui()?;

        gui.get_resize_hints(&mut self.plugin_handle())
    }

    fn adjust_size(&mut self, size: GuiSize) -> Option<GuiSize> {
        let gui = self.plugin_instance_gui()?;

        gui.adjust_size(&mut self.plugin_handle(), size)
    }

    fn set_size(&mut self, size: GuiSize) -> Result<(), PluginError> {
        let Some(gui) = self.plugin_instance_gui() else {
            return Err(PluginError::Message("Plugin does not support GUI"));
        };

        gui.set_size(&mut self.plugin_handle(), size)?;
        self.gui.size = Some(size);

        Ok(())
    }

    fn set_parent(&mut self, window: Window) -> Result<(), PluginError> {
        let Some(gui) = self.plugin_instance_gui() else {
            return Err(PluginError::Message("Plugin does not support GUI"));
        };

        let window = window.to_standard_api_type().unwrap(); // We don't support other API types yet

        // SAFETY: we are still within set_parent
        unsafe { gui.set_parent(&mut self.plugin_handle(), window)? };
        self.gui.parent = Some(window);

        Ok(())
    }

    fn set_transient(&mut self, window: Window) -> Result<(), PluginError> {
        let Some(gui) = self.plugin_instance_gui() else {
            return Err(PluginError::Message("Plugin does not support GUI"));
        };
        let window = window.to_standard_api_type().unwrap(); // We don't support other API types yet

        // SAFETY: we are still within set_transient
        unsafe { gui.set_transient(&mut self.plugin_handle(), window)? };
        self.gui.transient = Some(window);

        Ok(())
    }

    fn suggest_title(&mut self, title: &str) {
        let Some(gui) = self.plugin_instance_gui() else {
            return;
        };

        // FIXME
        let title = CString::new(title).unwrap();
        gui.suggest_title(&mut self.plugin_handle(), &title);
        self.gui.title = Some(title)
    }

    fn show(&mut self) -> Result<(), PluginError> {
        let Some(gui) = self.plugin_instance_gui() else {
            return Err(PluginError::Message("Plugin does not support GUI"));
        };

        gui.show(&mut self.plugin_handle())?;
        self.gui.shown = true;

        Ok(())
    }

    fn hide(&mut self) -> Result<(), PluginError> {
        let Some(gui) = self.plugin_instance_gui() else {
            return Err(PluginError::Message("Plugin does not support GUI"));
        };

        gui.hide(&mut self.plugin_handle())?;
        self.gui.shown = false;

        Ok(())
    }
}

#[derive(Copy, Clone)]
pub struct AtomicGuiSize {
    width: u32,
    height: NonZeroU32,
}

impl AtomicGuiSize {
    pub fn from_gui_size(size: GuiSize) -> Self {
        Self {
            height: NonZeroU32::new(size.height).unwrap_or(NonZeroU32::MAX),
            width: size.width,
        }
    }

    pub fn to_gui_size(self) -> GuiSize {
        let height = if self.height == NonZeroU32::MAX {
            0
        } else {
            self.height.get()
        };

        GuiSize {
            height,
            width: self.width,
        }
    }
}

impl HostGuiImpl for WrapperHostShared {
    fn resize_hints_changed(&self) {
        self.requests
            .gui
            .resize_hints_changed
            .store(true, Ordering::Relaxed)
    }

    fn request_resize(&self, new_size: GuiSize) -> Result<(), HostError> {
        self.requests
            .gui
            .resize_requested
            .store(Some(AtomicGuiSize::from_gui_size(new_size)));
        Ok(())
    }

    fn request_show(&self) -> Result<(), HostError> {
        self.requests
            .gui
            .show_requested
            .store(true, Ordering::Relaxed);

        Ok(())
    }

    fn request_hide(&self) -> Result<(), HostError> {
        self.requests
            .gui
            .hide_requested
            .store(true, Ordering::Relaxed);

        Ok(())
    }

    fn closed(&self, was_destroyed: bool) {
        if was_destroyed {
            self.requests.gui.destroyed.store(true, Ordering::Relaxed)
        } else {
            self.requests.gui.closed.store(true, Ordering::Relaxed)
        }
    }
}

pub struct PluginGuiRequests {
    resize_hints_changed: AtomicBool,
    resize_requested: AtomicCell<Option<AtomicGuiSize>>,
    // TODO: merge those bools?
    show_requested: AtomicBool,
    hide_requested: AtomicBool,
    closed: AtomicBool,
    destroyed: AtomicBool,
}

impl PluginGuiRequests {
    pub fn new() -> Self {
        Self {
            resize_hints_changed: AtomicBool::new(false),
            resize_requested: AtomicCell::new(None),
            show_requested: AtomicBool::new(false),
            hide_requested: AtomicBool::new(false),
            closed: AtomicBool::new(false),
            destroyed: AtomicBool::new(false),
        }
    }

    pub fn process_requests(
        &self,
        parent_host: &HostSharedHandle,
        extensions: &OuterHostExtensions,
    ) {
        let resize_hints_changed = self.resize_hints_changed.swap(false, Ordering::Relaxed);
        let gui_resize_requested = self.resize_requested.take();
        let show_requested = self.show_requested.swap(false, Ordering::Relaxed);
        let hide_requested = self.hide_requested.swap(false, Ordering::Relaxed);
        let closed = self.closed.swap(false, Ordering::Relaxed);
        let destroyed = self.destroyed.swap(false, Ordering::Relaxed);

        let Some(gui) = extensions.gui else {
            return;
        };

        // Special case: it makes no sense to call anything else when GUI was destroyed
        if destroyed {
            gui.closed(parent_host, true);
            return;
        }

        if let Some(size) = gui_resize_requested {
            let size = size.to_gui_size();
            let _ = gui.request_resize(parent_host, size.width, size.height); // TODO: handle errors
        }

        if show_requested {
            let _ = gui.request_show(parent_host);
        }

        if hide_requested {
            let _ = gui.request_hide(parent_host);
        }

        if resize_hints_changed {
            gui.resize_hints_changed(parent_host);
        }

        if closed {
            gui.closed(parent_host, false);
        }
    }
}
