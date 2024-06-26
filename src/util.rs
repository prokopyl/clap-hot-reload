use crate::_macro_utils::EntryDescriptor;
use clack_host::bundle::PluginBundle;
use clack_plugin::entry::EntryLoadError;
use libloading::Library;
use std::ffi::CStr;
use std::path::Path;

const fn cstr(bytes: &'static [u8]) -> &'static CStr {
    match CStr::from_bytes_with_nul(bytes) {
        Ok(str) => str,
        Err(_) => panic!(""),
    }
}

const WRAPPED_ENTRY_SYMBOL_NAME: &CStr = cstr(b"__clack_hotreload_wrapped_entry\0");

#[allow(unsafe_code)]
pub fn load_if_different_bundle(
    initial_entry: &EntryDescriptor,
    self_path: &Path,
) -> Result<Option<PluginBundle>, EntryLoadError> {
    let lib = unsafe { Library::new(self_path) }.map_err(|_| EntryLoadError)?;

    let symbol =
        unsafe { lib.get::<*mut EntryDescriptor>(WRAPPED_ENTRY_SYMBOL_NAME.to_bytes_with_nul()) }
            .map_err(|_| EntryLoadError)?;

    let loaded_entry: *mut EntryDescriptor = *symbol;
    if core::ptr::eq(initial_entry, loaded_entry) {
        return Ok(None);
    }

    let bundle = unsafe {
        PluginBundle::load_from_symbol_in_library(self_path, lib, WRAPPED_ENTRY_SYMBOL_NAME)
    }
    .map_err(|_| EntryLoadError)?;

    Ok(Some(bundle))
}
