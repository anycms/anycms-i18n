//! Hot-reload support for translation files.
//!
//! Watches a locale directory for file changes and reloads
//! translations in-place without restarting the application.
//! Works with any backend that implements [`Reloadable`].

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::core::Reloadable;
use crate::error::I18nError;

/// Watches a locale directory for file changes and hot-reloads
/// translations into the given [`Reloadable`] backend.
///
/// Since backends typically use `DashMap` internally, translations are
/// updated concurrently — no restart or rebuild needed.
///
/// The file extension to watch is determined by calling
/// [`Reloadable::file_extension`] on the backend.
///
/// Dropping the `HotReloader` stops the file watcher.
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use anycms_i18n::{TomlBackend, I18nBuilder, HotReloader};
///
/// let backend = Arc::new(TomlBackend::from_dir("locales")?);
/// let _reloader = HotReloader::watch("locales", backend.clone())?;
///
/// let i18n = I18nBuilder::new()
///     .default_locale("en")
///     .fallback_locale("en")
///     .add_backend(backend)
///     .build()?;
/// ```
pub struct HotReloader {
    _watcher: RecommendedWatcher,
    _thread: thread::JoinHandle<()>,
}

impl std::fmt::Debug for HotReloader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HotReloader").finish_non_exhaustive()
    }
}

impl HotReloader {
    /// Start watching a directory for translation file changes.
    ///
    /// The `backend` must be the same `Arc<dyn Reloadable>` passed to
    /// [`I18nBuilder::add_backend`](crate::I18nBuilder::add_backend).
    /// When files change, translations are reloaded in-place.
    pub fn watch(
        dir: impl AsRef<Path>,
        backend: Arc<dyn Reloadable>,
    ) -> Result<Self, I18nError> {
        let dir = dir.as_ref().canonicalize().map_err(|e| I18nError::IoError {
            path: dir.as_ref().to_path_buf(),
            source: e,
        })?;

        if !dir.is_dir() {
            return Err(I18nError::IoError {
                path: dir.clone(),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "locale directory not found",
                ),
            });
        }

        let extension = backend.file_extension().to_string();

        let (tx, rx) = std::sync::mpsc::channel::<Event>();

        let mut watcher =
            RecommendedWatcher::new(
                move |res: Result<Event, notify::Error>| {
                    if let Ok(event) = res {
                        let _ = tx.send(event);
                    }
                },
                Config::default(),
            )
            .map_err(|e| I18nError::WatchError(e.to_string()))?;

        watcher
            .watch(&dir, RecursiveMode::NonRecursive)
            .map_err(|e| I18nError::WatchError(e.to_string()))?;

        let watch_dir = dir;
        let handle = thread::spawn(move || {
            loop {
                // Wait for first event
                let Ok(event) = rx.recv() else { break };

                // Collect changed paths
                let mut pending: HashSet<PathBuf> = HashSet::new();
                collect_paths(&event, &extension, &mut pending);

                // Drain queued events (debounce window)
                while let Ok(event) = rx.recv_timeout(Duration::from_millis(200)) {
                    collect_paths(&event, &extension, &mut pending);
                }

                // Reload each changed file
                for path in &pending {
                    reload_file(&watch_dir, &*backend, path);
                }
            }
        });

        Ok(Self {
            _watcher: watcher,
            _thread: handle,
        })
    }
}

/// Collect file paths matching the expected extension from a notify event.
fn collect_paths(event: &Event, extension: &str, out: &mut HashSet<PathBuf>) {
    let relevant = matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    );
    if !relevant {
        return;
    }

    for path in &event.paths {
        if path.extension().and_then(|e| e.to_str()) == Some(extension) {
            out.insert(path.clone());
        }
    }
}

/// Reload a single locale file into the backend.
fn reload_file(dir: &Path, backend: &dyn Reloadable, path: &Path) {
    let locale = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    // Skip if file is outside our watched dir (shouldn't happen, but be safe)
    if !path.starts_with(dir) {
        return;
    }

    match std::fs::read_to_string(path) {
        Ok(content) => match backend.reload_from_str(locale, &content) {
            Ok(()) => tracing::info!(locale, path = %path.display(), "hot-reloaded locale"),
            Err(e) => {
                tracing::warn!(locale, error = %e, "failed to parse locale file on hot-reload")
            }
        },
        Err(e) => {
            // File might have been removed — this is fine
            if path.exists() {
                tracing::warn!(path = %path.display(), error = %e, "failed to read locale file");
            }
        }
    }
}
