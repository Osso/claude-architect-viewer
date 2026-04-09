mod chat;
mod message;
mod sidebar;
mod state;
mod watcher;

use dioxus::prelude::*;
use state::{ChatMessage, ProjectEntry};

fn desktop_config() -> dioxus::desktop::Config {
    dioxus::desktop::Config::new()
        .with_menu(None)
        .with_custom_head(format!(
            "<style>{}</style>",
            include_str!("../assets/style.css")
        ))
        .with_window(
            dioxus::desktop::tao::window::WindowBuilder::new()
                .with_decorations(false)
                .with_title("Architect Viewer"),
        )
}

fn load_project_messages(proj: &ProjectEntry, offset: u64) -> (Vec<ChatMessage>, u64) {
    match &proj.jsonl_path {
        Some(path) => {
            let (msgs, off, _) = state::parse_jsonl_from_offset(path, offset);
            (msgs, off)
        }
        None => (Vec::new(), 0),
    }
}

fn find_project<'a>(projects: &'a [ProjectEntry], name: &str) -> Option<&'a ProjectEntry> {
    projects.iter().find(|p| p.name == name)
}

fn session_changed(old: &[ProjectEntry], new: &[ProjectEntry], name: &str) -> bool {
    let old_id = find_project(old, name).map(|p| p.session_id.as_str());
    let new_id = find_project(new, name).map(|p| p.session_id.as_str());
    old_id != new_id
}

fn handle_sessions_changed(
    new_projects: Vec<ProjectEntry>,
    mut projects: Signal<Vec<ProjectEntry>>,
    mut messages: Signal<Vec<ChatMessage>>,
    mut offset: Signal<u64>,
    selected: Signal<Option<String>>,
) {
    let sel = selected.read().clone();
    if let Some(ref name) = sel {
        if session_changed(&projects.read(), &new_projects, name) {
            messages.set(Vec::new());
            offset.set(0);
            if let Some(proj) = find_project(&new_projects, name) {
                let (msgs, new_off) = load_project_messages(proj, 0);
                messages.set(msgs);
                offset.set(new_off);
            }
        }
    }
    projects.set(new_projects);
}

fn handle_jsonl_changed(
    path: std::path::PathBuf,
    projects: Signal<Vec<ProjectEntry>>,
    mut messages: Signal<Vec<ChatMessage>>,
    mut offset: Signal<u64>,
    selected: Signal<Option<String>>,
) {
    let sel = selected.read().clone();
    let Some(ref name) = sel else { return };
    let proj = find_project(&projects.read(), name).cloned();
    let Some(proj) = proj else { return };
    if proj.jsonl_path.as_ref() != Some(&path) {
        return;
    }
    let cur_offset = *offset.read();
    let (new_msgs, new_off, had_reset) = state::parse_jsonl_from_offset(&path, cur_offset);
    if had_reset {
        messages.set(new_msgs);
    } else if !new_msgs.is_empty() {
        messages.write().extend(new_msgs);
    }
    offset.set(new_off);
}

fn setup_selection_effect(
    selected: Signal<Option<String>>,
    projects: Signal<Vec<ProjectEntry>>,
    mut messages: Signal<Vec<ChatMessage>>,
    mut offset: Signal<u64>,
) {
    use_effect(move || {
        let sel = selected.read().clone();
        messages.set(Vec::new());
        offset.set(0);
        let Some(ref name) = sel else { return };
        let proj = find_project(&projects.read(), name).cloned();
        let Some(proj) = proj else { return };
        let (msgs, new_off) = load_project_messages(&proj, 0);
        messages.set(msgs);
        offset.set(new_off);
    });
}

fn setup_watcher_future(
    projects: Signal<Vec<ProjectEntry>>,
    messages: Signal<Vec<ChatMessage>>,
    offset: Signal<u64>,
    selected: Signal<Option<String>>,
) {
    use_future(move || async move {
        let mut rx = spawn_watcher_bridge();
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            drain_watch_events(&mut rx, projects, messages, offset, selected);
        }
    });
}

fn spawn_watcher_bridge() -> tokio::sync::mpsc::UnboundedReceiver<watcher::WatchEvent> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    std::thread::spawn(move || {
        let std_rx = watcher::start_watcher();
        while let Ok(event) = std_rx.recv() {
            if tx.send(event).is_err() {
                break;
            }
        }
    });
    rx
}

fn drain_watch_events(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<watcher::WatchEvent>,
    projects: Signal<Vec<ProjectEntry>>,
    messages: Signal<Vec<ChatMessage>>,
    offset: Signal<u64>,
    selected: Signal<Option<String>>,
) {
    while let Ok(event) = rx.try_recv() {
        apply_watch_event(event, projects, messages, offset, selected);
    }
}

fn apply_watch_event(
    event: watcher::WatchEvent,
    projects: Signal<Vec<ProjectEntry>>,
    messages: Signal<Vec<ChatMessage>>,
    offset: Signal<u64>,
    selected: Signal<Option<String>>,
) {
    match event {
        watcher::WatchEvent::SessionsChanged => {
            let new_projects = state::load_sessions();
            handle_sessions_changed(new_projects, projects, messages, offset, selected);
        }
        watcher::WatchEvent::JsonlChanged(path) => {
            handle_jsonl_changed(path, projects, messages, offset, selected);
        }
    }
}

fn main() {
    tracing_subscriber::fmt::init();
    dioxus::LaunchBuilder::desktop()
        .with_cfg(desktop_config())
        .launch(App);
}

#[component]
fn App() -> Element {
    let mut projects = use_context_provider(|| Signal::new(Vec::<ProjectEntry>::new()));
    let selected = use_context_provider(|| Signal::new(Option::<String>::None));
    let messages = use_context_provider(|| Signal::new(Vec::<ChatMessage>::new()));
    let offset = use_context_provider(|| Signal::new(0u64));

    use_effect(move || {
        projects.set(state::load_sessions());
    });

    setup_selection_effect(selected, projects, messages, offset);
    setup_watcher_future(projects, messages, offset, selected);

    rsx! {
        div { class: "app",
            div { class: "drag-region" }
            div { class: "app-body",
                sidebar::Sidebar {}
                chat::ConversationPane {}
            }
        }
    }
}
