#![cfg_attr(coverage_nightly, coverage(off))]

use crate::state::{ChatMessage, ProjectEntry};
use dioxus::prelude::*;

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
                    let (msgs, new_off, _) = crate::state::parse_jsonl_from_offset(path, 0);
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
    let active_project = selected.read().clone();
    let project_values = projects.read().clone();

    rsx! {
        div { class: "sidebar",
            div { class: "sidebar-header", "PROJECTS" }
            SidebarList { projects: project_values, active_project }
        }
    }
}

#[component]
fn SidebarList(projects: Vec<ProjectEntry>, active_project: Option<String>) -> Element {
    let rows = projects.into_iter().map(|project| {
        rsx! {
            ProjectRow {
                key: "{project.name}",
                is_active: active_project.as_deref() == Some(project.name.as_str()),
                project,
            }
        }
    });

    rsx! {
        div { class: "sidebar-list",
            {rows}
        }
    }
}
