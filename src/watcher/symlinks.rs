use notify_debouncer_full::notify::{RecursiveMode, Watcher};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct BundleSymlinkedPath {
    path: PathBuf,
    _is_symlink: bool, // TODO
    is_watched: bool,
    is_parent_watched: bool,
    symlink_target: Option<Box<BundleSymlinkedPath>>,
}

impl BundleSymlinkedPath {
    pub fn path(&self) -> &Path {
        &self.path
    }

    // TODO: handle symlink loops
    pub fn get_info(path: PathBuf) -> Self {
        dbg!("Scanning path:  ", &path);

        let (is_symlink, symlink_target) = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if !metadata.is_symlink() {
                    (false, None)
                } else {
                    match std::fs::read_link(&path) {
                        Ok(path) => (true, Some(Box::new(BundleSymlinkedPath::get_info(path)))),
                        Err(_) => {
                            // TODO: log error
                            (true, None)
                        }
                    }
                }
            }
            Err(_) => {
                // TODO: log error
                match std::fs::read_link(&path) {
                    Ok(path) => (true, Some(Box::new(BundleSymlinkedPath::get_info(path)))),
                    Err(_) => (false, None),
                }
            }
        };

        Self {
            path,
            is_watched: false,
            is_parent_watched: false,
            _is_symlink: is_symlink,
            symlink_target,
        }
    }

    pub fn watch_all(&mut self, watcher: &mut impl Watcher, results: &mut WatchResults) {
        match watcher.watch(&self.path, RecursiveMode::NonRecursive) {
            Ok(()) => {
                println!("Started watching path {:?}", &self.path);
                self.is_watched = true;
                results.success_count += 1;
            }
            Err(e) => {
                eprintln!("Failed to watch path {:?}", &self.path);
                self.is_watched = false;
                results.errors.push(e)
            }
        }

        if let Some(parent_path) = self.path.parent() {
            match watcher.watch(parent_path, RecursiveMode::NonRecursive) {
                Ok(()) => {
                    println!("Started watching parent path {:?}", parent_path);
                    self.is_parent_watched = true;
                    results.success_count += 1;
                }
                Err(e) => {
                    eprintln!("Failed to watch parent path {:?}", parent_path);
                    self.is_parent_watched = false;
                    results.errors.push(e)
                }
            }
        }

        if let Some(target) = self.symlink_target.as_mut() {
            target.watch_all(watcher, results)
        }
    }

    pub fn iter(&self) -> BundleSymlinkedPathIter {
        BundleSymlinkedPathIter {
            current: Some(self),
        }
    }
}

pub struct BundleSymlinkedPathIter<'a> {
    current: Option<&'a BundleSymlinkedPath>,
}

impl<'a> Iterator for BundleSymlinkedPathIter<'a> {
    type Item = &'a BundleSymlinkedPath;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(current) = self.current.take() else {
            return None;
        };

        self.current = current.symlink_target.as_deref();

        Some(current)
    }
}

pub struct WatchResults {
    success_count: usize,
    errors: Vec<notify_debouncer_full::notify::Error>,
}

impl WatchResults {
    pub fn empty() -> Self {
        WatchResults {
            success_count: 0,
            errors: Vec::new(),
        }
    }

    pub fn has_any_success(&self) -> bool {
        self.success_count > 0
    }

    pub fn log_errors(&self) {
        // TODO
    }
}
