use crate::watcher::event_thread::WatcherEventThread;
use crate::watcher::symlinks::{BundleSymlinkedPath, WatchResults};
use clack_host::bundle::*;
use notify_debouncer_full::notify::{RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, Debouncer, FileIdMap};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::Duration;

// TODO: bikeshed
mod event_thread;
mod inner;
mod symlinks;

// TODO: bikeshed
pub struct WatcherMaster {
    initial_bundle: PluginBundle,
    notifier: Option<Debouncer<RecommendedWatcher, FileIdMap>>,
}

impl WatcherMaster {
    pub fn new(initial_bundle: PluginBundle, bundle_path: &Path) -> Self {
        let mut path = BundleSymlinkedPath::get_info(bundle_path.to_path_buf());

        let notifier = new_debouncer(
            Duration::from_millis(750),
            None,
            WatcherEventThread::new(path.clone()),
        );

        let notifier = match notifier {
            Ok(mut w) => {
                let mut results = WatchResults::empty();
                path.watch_all(w.watcher(), &mut results);
                results.log_errors();

                results.has_any_success().then_some(w)
            }
            Err(e) => {
                eprintln!("[CLAP PLUGIN HOT RELOADER] Failed to start file watcher: {e}");
                None
            }
        };

        Self {
            initial_bundle,
            notifier,
        }
    }

    pub fn initial_bundle(&self) -> &PluginBundle {
        &self.initial_bundle
    }

    pub fn create_handle<'a>(&self, callback: impl Fn() + Send + Sync + 'a) -> WatcherHandle<'a> {
        WatcherHandle {
            current_bundle: self.initial_bundle.clone(),
            callback: Box::new(callback),
        }
    }
}

pub struct WatcherHandle<'a> {
    current_bundle: PluginBundle,
    callback: Box<dyn Fn() + Send + Sync + 'a>,
}

impl<'a> WatcherHandle<'a> {
    pub fn current_bundle(&self) -> &PluginBundle {
        &self.current_bundle
    }

    pub fn check_new_bundle_available(&self) -> Option<&PluginBundle> {
        todo!()
    }
}
