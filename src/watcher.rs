use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;

#[derive(Debug)]
pub enum WatchEvent {
    SessionsChanged,
    JsonlChanged(PathBuf),
}

pub fn start_watcher() -> std_mpsc::Receiver<WatchEvent> {
    let (tx, rx) = std_mpsc::channel::<WatchEvent>();

    std::thread::spawn(move || {
        let (notify_tx, notify_rx) = std_mpsc::channel::<notify::Result<Event>>();

        let mut watcher = match RecommendedWatcher::new(notify_tx, Config::default()) {
            Ok(w) => w,
            Err(e) => {
                tracing::error!("Failed to create file watcher: {}", e);
                return;
            }
        };

        watch_sessions_file(&mut watcher);
        watch_projects_dir(&mut watcher);

        loop {
            match notify_rx.recv() {
                Ok(Ok(event)) => {
                    if let Some(watch_event) = classify_event(event) {
                        if tx.send(watch_event).is_err() {
                            break;
                        }
                    }
                }
                Ok(Err(e)) => {
                    tracing::warn!("Watch error: {}", e);
                }
                Err(_) => break,
            }
        }
    });

    rx
}

fn sessions_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("claude-architect").join("sessions.json"))
}

fn projects_path() -> Option<PathBuf> {
    dirs::home_dir().map(|d| d.join(".claude").join("projects"))
}

fn watch_sessions_file(watcher: &mut RecommendedWatcher) {
    let path = match sessions_path() {
        Some(p) => p,
        None => {
            tracing::warn!("Could not resolve sessions.json path");
            return;
        }
    };

    if !path.exists() {
        tracing::warn!("sessions.json not found at {}, skipping watch", path.display());
        return;
    }

    if let Err(e) = watcher.watch(&path, RecursiveMode::NonRecursive) {
        tracing::warn!("Failed to watch {}: {}", path.display(), e);
    }
}

fn watch_projects_dir(watcher: &mut RecommendedWatcher) {
    let path = match projects_path() {
        Some(p) => p,
        None => {
            tracing::warn!("Could not resolve ~/.claude/projects path");
            return;
        }
    };

    if !path.exists() {
        tracing::warn!(
            "~/.claude/projects not found at {}, skipping watch",
            path.display()
        );
        return;
    }

    if let Err(e) = watcher.watch(&path, RecursiveMode::Recursive) {
        tracing::warn!("Failed to watch {}: {}", path.display(), e);
    }
}

fn classify_event(event: Event) -> Option<WatchEvent> {
    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {}
        _ => return None,
    }

    for path in &event.paths {
        if is_sessions_file(path) {
            return Some(WatchEvent::SessionsChanged);
        }

        if is_jsonl_file(path) {
            return Some(WatchEvent::JsonlChanged(path.clone()));
        }
    }

    None
}

fn is_sessions_file(path: &std::path::Path) -> bool {
    sessions_path()
        .map(|p| path == p)
        .unwrap_or(false)
}

fn is_jsonl_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e == "jsonl")
        .unwrap_or(false)
}
