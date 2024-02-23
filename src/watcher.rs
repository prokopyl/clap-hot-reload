use clack_host::bundle::*;
use clack_plugin::entry::EntryLoadError;
use libloading::Library;
use std::ffi::{CStr, OsStr};
use std::ops::Deref;

// TODO: bikeshed
mod inner;

// TODO: bikeshed
pub struct WatcherMaster {
    initial_bundle: PluginBundle,
}

const WRAPPED_ENTRY_SYMBOL_NAME: &CStr =
    unsafe { CStr::from_bytes_with_nul_unchecked(b"__clack_hotreload_wrapped_entry\0") };

fn do_load_bikeshed_me(
    initial_entry: &EntryDescriptor,
    self_path: &CStr,
) -> Result<Option<PluginBundle>, EntryLoadError> {
    let self_path = self_path.to_str().map_err(|_| EntryLoadError)?;
    let lib = unsafe { Library::new(self_path) }.map_err(|_| EntryLoadError)?;

    let symbol =
        unsafe { lib.get::<*mut EntryDescriptor>(WRAPPED_ENTRY_SYMBOL_NAME.to_bytes_with_nul()) }
            .map_err(|_| EntryLoadError)?;

    let loaded_entry: &*mut EntryDescriptor = symbol.deref();
    if core::ptr::eq(initial_entry, *loaded_entry) {
        return Ok(None);
    }

    let bundle = unsafe {
        PluginBundle::load_from_symbol_in_library(self_path, lib, WRAPPED_ENTRY_SYMBOL_NAME)
    }
    .map_err(|_| EntryLoadError)?;

    Ok(Some(bundle))
}

impl WatcherMaster {
    pub fn new(
        initial_entry: &'static EntryDescriptor,
        self_path: &CStr,
    ) -> Result<Self, EntryLoadError> {
        let bundle =
            if let Ok(Some(different_bundle)) = do_load_bikeshed_me(initial_entry, self_path) {
                different_bundle
            } else {
                // TODO: double utf8 check
                let self_path = self_path.to_str().map_err(|_| EntryLoadError)?;
                unsafe { PluginBundle::load_from_raw(initial_entry, self_path) }
                    .map_err(|_| EntryLoadError)?
            };

        Ok(Self {
            initial_bundle: bundle,
        })
    }

    pub fn initial_bundle(&self) -> &PluginBundle {
        &self.initial_bundle
    }

    pub fn create_handle<'a>(&self, callback: impl Fn() + Send + Sync + 'a) -> WatcherHandle<'a> {
        // TODO
        WatcherHandle {
            callback: Box::new(callback),
        }
    }
}

pub struct WatcherHandle<'a> {
    callback: Box<dyn Fn() + Send + Sync + 'a>,
}

impl<'a> WatcherHandle<'a> {
    pub fn current_bundle(&self) -> &PluginBundle {
        todo!()
    }

    pub fn check_new_bundle_available(&self) -> Option<&PluginBundle> {
        todo!()
    }
}
