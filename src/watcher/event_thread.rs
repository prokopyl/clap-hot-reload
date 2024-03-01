use crate::util::load_if_different_bundle;
use crate::watcher::symlinks::BundleSymlinkedPath;
use clack_host::prelude::PluginBundle;
use notify_debouncer_full::notify::Error;
use notify_debouncer_full::{DebounceEventHandler, DebounceEventResult, DebouncedEvent};

pub struct WatcherEventThread {
    bundle_path: BundleSymlinkedPath,
    current_bundle: PluginBundle,
}

impl DebounceEventHandler for WatcherEventThread {
    fn handle_event(&mut self, event: DebounceEventResult) {
        match event {
            Ok(events) => self.handle_updates(events),
            Err(errors) => self.handle_errors(errors),
        }
    }
}

impl WatcherEventThread {
    pub fn new(bundle_path: BundleSymlinkedPath, initial_bundle: PluginBundle) -> Self {
        println!("New event thread reload started");
        Self {
            bundle_path,
            current_bundle: initial_bundle,
        }
    }

    fn handle_errors(&mut self, errors: Vec<Error>) {
        // TODO
        eprintln!("Watcher errors: {errors:?}")
    }

    fn handle_updates(&mut self, updates: Vec<DebouncedEvent>) {
        // TODO: for now, we ignore updating the symlink list, we just watch for events.

        let something_updated = updates
            .iter()
            .flat_map(|event| &event.paths)
            .any(|event_path| {
                self.bundle_path
                    .iter()
                    .any(|watched_path| watched_path.path() == event_path)
            });

        if !something_updated {
            return;
        }

        println!("File changed! : {updates:?}");

        let final_file = self.bundle_path.iter().last().unwrap().path();

        let new_bundle = match load_if_different_bundle(self.current_bundle.raw_entry(), final_file)
        {
            Ok(Some(bundle)) => bundle,
            Ok(None) => {
                println!("File changed but points to same bundle, not reloading plugins.");
                return;
            }
            Err(e) => {
                eprintln!("Failed to hot-load new bundle: {e}");
                return;
            }
        };

        println!("New bundle found.  CLAP version: {}", new_bundle.version());
        self.current_bundle = new_bundle;
    }
}
