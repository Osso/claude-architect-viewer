use dioxus::prelude::*;
use crate::state::{ChatMessage, ProjectEntry};

#[component]
fn ProjectRow(project: ProjectEntry, is_active: bool) -> Element {
    let mut selected = use_context::<Signal<Option<String>>>();
    let mut messages = use_context::<Signal<Vec<ChatMessage>>>();
    let mut offset = use_context::<Signal<u64>>();
    let name = project.name.clone();
    let jsonl_path = project.jsonl_path.clone();

    rsx! {
        div {
            class: if is_active { "session-item active" } else { "session-item" },
            onclick: move |_| {
                selected.set(Some(name.clone()));
                messages.set(Vec::new());
                offset.set(0);
                if let Some(ref path) = jsonl_path {
                    let (msgs, new_off) = crate::state::parse_jsonl_from_offset(path, 0);
                    messages.set(msgs);
                    offset.set(new_off);
                }
            },
            span { class: "session-item-title", "{project.name}" }
            span { class: "badge badge-count", "{project.validations}" }
        }
    }
}

#[component]
pub fn Sidebar() -> Element {
    let projects = use_context::<Signal<Vec<ProjectEntry>>>();
    let selected = use_context::<Signal<Option<String>>>();

    rsx! {
        div { class: "sidebar",
            div { class: "sidebar-header", "PROJECTS" }
            div { class: "sidebar-list",
                for project in projects.read().clone().into_iter() {
                    {
                        let is_active = selected.read().as_deref() == Some(project.name.as_str());
                        rsx! {
                            ProjectRow {
                                key: "{project.name}",
                                project: project,
                                is_active: is_active,
                            }
                        }
                    }
                }
            }
        }
    }
}
