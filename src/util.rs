use crate::_macro_utils::EntryDescriptor;
use clack_host::bundle::PluginBundle;
use clack_plugin::entry::EntryLoadError;
use libloading::Library;
use std::ffi::CStr;
use std::path::Path;

const WRAPPED_ENTRY_SYMBOL_NAME: &CStr =
    unsafe { CStr::from_bytes_with_nul_unchecked(b"__clack_hotreload_wrapped_entry\0") };

const FOO_SYMBOL_NAME: &CStr =
    unsafe { CStr::from_bytes_with_nul_unchecked(b"__clack_hotreload_foo\0") };

pub fn load_if_different_bundle(
    initial_entry: &EntryDescriptor,
    self_path: &Path,
) -> Result<Option<PluginBundle>, EntryLoadError> {
    let lib = unsafe { Library::new(self_path) }.map_err(|_| EntryLoadError)?;

    let foo_symbol = unsafe { lib.get::<*mut u32>(FOO_SYMBOL_NAME.to_bytes_with_nul()) }
        .map_err(|_| EntryLoadError)?;

    let foo = unsafe { **foo_symbol };
    println!("Library FOO value: {foo}");

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
