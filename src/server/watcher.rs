use std::path::PathBuf;
use std::time::Duration;

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

use crate::config::AppConfig;

const DEBOUNCE_WINDOW: Duration = Duration::from_millis(500);

/// Start watching `target_dir` for file changes and trigger a debounced index
/// refresh after each burst of events. The returned `RecommendedWatcher` must
/// be kept alive for as long as watching is needed.
pub fn spawn_watcher(
    target_dir: PathBuf,
    db_path: PathBuf,
    config: AppConfig,
) -> anyhow::Result<RecommendedWatcher> {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<notify::Event>();

    let mut watcher =
        notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        let _ = event_tx.send(event);
                    }
                    _ => {}
                }
            }
        })?;

    watcher.watch(&target_dir, RecursiveMode::Recursive)?;

    tokio::spawn(async move {
        loop {
            // Wait for the first event of a burst
            if event_rx.recv().await.is_none() {
                break;
            }

            // Debounce: let the burst settle before triggering a refresh
            tokio::time::sleep(DEBOUNCE_WINDOW).await;
            while event_rx.try_recv().is_ok() {}

            tracing::info!("File change detected — triggering index refresh");

            // Await directly so a slow refresh naturally gates the next one;
            // no concurrent refresh tasks can pile up.
            match crate::indexer::refresh(&config, &db_path, &target_dir).await {
                Ok((n, _)) if n > 0 => tracing::info!("Background refresh: {n} file(s) updated"),
                Ok(_) => {}
                Err(e) => tracing::error!("Background refresh failed: {e}"),
            }
        }
    });

    Ok(watcher)
}
