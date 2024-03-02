use crate::watcher::event_thread::WatcherEventThread;
use crate::watcher::symlinks::{BundleSymlinkedPath, WatchResults};
use clack_host::bundle::*;
use notify_debouncer_full::notify::{RecommendedWatcher, Watcher};
use notify_debouncer_full::{new_debouncer, Debouncer, FileIdMap};
use std::path::Path;
use std::time::Duration;

// TODO: bikeshed
mod event_thread;
mod symlinks;

// TODO: bikeshed
mod fanout;
pub use fanout::*;

// TODO: bikeshed
pub struct WatcherMaster {
    notifier: Debouncer<RecommendedWatcher, FileIdMap>,
    factory: BundleReceiverFactory,
}

impl WatcherMaster {
    pub fn new(initial_bundle: PluginBundle, bundle_path: &Path) -> Option<Self> {
        let mut path = BundleSymlinkedPath::get_info(bundle_path.to_path_buf());

        let (producer, factory) = new_bundle_fanout(initial_bundle.clone());

        let notifier = new_debouncer(
            Duration::from_millis(750),
            None,
            WatcherEventThread::new(path.clone(), initial_bundle, producer),
        );

        let notifier = match notifier {
            Ok(mut w) => {
                let mut results = WatchResults::empty();
                path.watch_all(w.watcher(), &mut results);
                results.log_errors();

                results.has_any_success().then_some(w)?
            }
            Err(e) => {
                eprintln!("[CLAP PLUGIN HOT RELOADER] Failed to start file watcher: {e}");
                return None;
            }
        };

        Some(Self { notifier, factory })
    }

    pub fn new_receiver(&self) -> BundleReceiver {
        self.factory.new_receiver()
    }
}
