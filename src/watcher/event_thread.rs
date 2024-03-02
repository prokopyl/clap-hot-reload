use crate::util::load_if_different_bundle;
use crate::watcher::symlinks::BundleSymlinkedPath;
use crate::watcher::BundleProducer;
use blake3::{Hash, Hasher};
use clack_host::prelude::PluginBundle;
use clack_plugin::prelude::EntryDescriptor;
use notify_debouncer_full::notify::Error;
use notify_debouncer_full::{DebounceEventHandler, DebounceEventResult, DebouncedEvent};
use std::fs::File;
use std::io;
use std::io::{BufReader, Write};
use std::path::Path;
use tempfile::NamedTempFile;

struct PluginBundleFile {
    bundle: PluginBundle,
    file_hash: Option<Hash>,
    temp_file: Option<NamedTempFile>,
}

impl PluginBundleFile {
    pub fn new_fileless(bundle: PluginBundle) -> Self {
        Self {
            bundle,
            file_hash: None,
            temp_file: None,
        }
    }

    pub fn new_compare_to_hash(
        path: &Path,
        current_hash: Option<Hash>,
        current_entry: &EntryDescriptor,
    ) -> io::Result<Option<Self>> {
        // First compare hashes, skip if hashes are identical, or if compute failed for some reason.
        let file_hash = match compute_hash(path) {
            Ok(h) => {
                if let Some(hash) = current_hash {
                    if hash == h {
                        return Ok(None);
                    }
                }

                Some(h)
            }
            Err(e) => {
                eprintln!("Failed to compute hash for {path:?}: {e}");
                None
            }
        };

        // Now copy to a tempfile
        let tempfile = create_tempfile_copy(path)?;

        let bundle = match load_if_different_bundle(current_entry, tempfile.path()) {
            Ok(Some(bundle)) => bundle,
            Ok(None) => {
                println!("File changed but points to same bundle, not reloading plugins.");
                return Ok(None);
            }
            Err(e) => {
                eprintln!("Failed to hot-load new bundle : {e}");
                return Err(io::Error::other(e));
            }
        };

        Ok(Some(Self {
            bundle,
            file_hash,
            temp_file: Some(tempfile),
        }))
    }
}

impl Drop for PluginBundleFile {
    fn drop(&mut self) {
        if let Some(file) = self.temp_file.take() {
            let path = file.path().to_path_buf();
            if let Err(e) = file.close() {
                eprintln!("Failed to remove temp file {path:?}: {e}")
            }
        }
    }
}

pub struct WatcherEventThread {
    bundle_path: BundleSymlinkedPath,
    current_bundle: PluginBundleFile,
    producer: BundleProducer,
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
    pub fn new(
        bundle_path: BundleSymlinkedPath,
        initial_bundle: PluginBundle,
        producer: BundleProducer,
    ) -> Self {
        println!("New event thread reload started");
        Self {
            bundle_path,
            current_bundle: PluginBundleFile::new_fileless(initial_bundle),
            producer,
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

        let bundle_file = self.bundle_path.path();

        let new_bundle = PluginBundleFile::new_compare_to_hash(
            bundle_file,
            self.current_bundle.file_hash,
            self.current_bundle.bundle.raw_entry(),
        );

        let new_bundle = match new_bundle {
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

        println!(
            "New bundle found.  CLAP version: {}",
            new_bundle.bundle.version()
        );
        self.current_bundle = new_bundle;
        self.producer.produce(&self.current_bundle.bundle);
    }
}

fn compute_hash(path: &Path) -> io::Result<Hash> {
    const BUFFER_SIZE: usize = 1024 * 1024; // 1MiB buffer

    let file = File::open(path)?;
    let reader = BufReader::with_capacity(BUFFER_SIZE, file);

    let mut hasher = Hasher::new();
    hasher.update_reader(reader)?;
    Ok(hasher.finalize())
}

// TODO: more resilient errors
fn create_tempfile_copy(path: &Path) -> io::Result<NamedTempFile> {
    let mut file = File::open(path)?;
    let mut temp_file = NamedTempFile::new()?;

    std::io::copy(&mut file, temp_file.as_file_mut())?;
    temp_file.as_file_mut().flush()?;

    Ok(temp_file)
}
