extern crate core;

mod entry;
mod util;
mod watcher;
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
                #[allow(non_upper_case_globals, missing_docs)]
                #[allow(unsafe_code)]
                #[allow(warnings, unused)]
                #[no_mangle]
                pub static __clack_hotreload_wrapped_entry: $crate::_macro_utils::EntryDescriptor =
                    $entry_value;

                #[allow(non_upper_case_globals, missing_docs)]
                #[allow(unsafe_code)]
                #[allow(warnings, unused)]
                #[no_mangle]
                pub static __clack_hotreload_foo: u32 = 42699;

                |p| $crate::_macro_utils::HotReloaderEntry::new(p, &__clack_hotreload_wrapped_entry)
            })
        );
    };
}
