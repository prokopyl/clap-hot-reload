use crate::watcher::symlinks::BundleSymlinkedPath;
use notify_debouncer_full::notify::Error;
use notify_debouncer_full::{DebounceEventHandler, DebounceEventResult, DebouncedEvent};

pub struct WatcherEventThread {
    bundle_path: BundleSymlinkedPath,
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
    pub fn new(bundle_path: BundleSymlinkedPath) -> Self {
        println!("New event thread reload started");
        Self { bundle_path }
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

        if something_updated {
            println!("File changed! : {updates:?}")
        }
    }
}
