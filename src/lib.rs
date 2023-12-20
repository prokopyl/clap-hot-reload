extern crate core;

mod entry;
mod wrapper;

#[doc(hidden)]
pub mod _macro_utils {
    pub use crate::entry::HotReloaderEntry;
    pub use clack_plugin::clack_export_entry;
    pub use clack_plugin::entry::EntryDescriptor;
}

#[macro_export]
macro_rules! export_reloadable_clap_entry {
    ($entry_value:expr) => {
        $crate::_macro_utils::clack_export_entry!(
            $crate::_macro_utils::HotReloaderEntry,
            ({
                static WRAPPED_ENTRY: $crate::_macro_utils::EntryDescriptor = $entry_value;
                |p| $crate::_macro_utils::HotReloaderEntry::new(p, &WRAPPED_ENTRY)
            })
        );
    };
}
