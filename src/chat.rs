use crate::message::MessageRow;
use crate::state::{ChatMessage, ProjectEntry, TokenUsage};
use dioxus::prelude::*;

fn sum_token_usage(messages: &[ChatMessage]) -> Option<TokenUsage> {
    let usages: Vec<&TokenUsage> = messages
        .iter()
        .filter_map(|m| match m {
            ChatMessage::Assistant { usage: Some(u), .. } => Some(u),
            _ => None,
        })
        .collect();

    if usages.is_empty() {
        return None;
    }

    Some(TokenUsage {
        input: usages.iter().map(|u| u.input).sum(),
        output: usages.iter().map(|u| u.output).sum(),
        cache_read: usages.iter().map(|u| u.cache_read).sum(),
        cache_creation: usages.iter().map(|u| u.cache_creation).sum(),
    })
}

#[component]
fn SessionHeader(name: String, session_id: String, validations: u32) -> Element {
    let short_id = if session_id.len() >= 8 {
        session_id[..8].to_string()
    } else {
        session_id.clone()
    };
    rsx! {
        div { class: "session-header",
            span { "{name}" }
            span { class: "badge", "session: {short_id}" }
            span { class: "badge badge-count", "{validations} validations" }
        }
    }
}

#[component]
fn CumulativeTokenBar(usage: TokenUsage) -> Element {
    rsx! {
        div { class: "token-bar-cumulative",
            span { class: "token-badge token-input", "in: {usage.input}" }
            span { class: "token-badge token-output", "out: {usage.output}" }
            span { class: "token-badge token-cache-read", "cache_r: {usage.cache_read}" }
            span { class: "token-badge token-cache-write", "cache_w: {usage.cache_creation}" }
        }
    }
}

#[component]
fn MessageList(msgs: Vec<ChatMessage>) -> Element {
    let msg_count = msgs.len();
    use_effect(move || {
        let _ = msg_count;
        document::eval(
            "let el = document.getElementById('message-list'); if (el) el.scrollTop = el.scrollHeight;",
        );
    });
    rsx! {
        div { id: "message-list", class: "message-list",
            MessageRows { msgs }
        }
    }
}

#[component]
fn MessageRows(msgs: Vec<ChatMessage>) -> Element {
    if msgs.is_empty() {
        return rsx! {
            div { class: "chat-empty", "No messages yet" }
        };
    }
    rsx! {
        for (i, msg) in msgs.into_iter().enumerate() {
            MessageRow { key: "{i}", msg }
        }
    }
}

fn resolve_selected_project(
    selected: &Signal<Option<String>>,
    projects: &Signal<Vec<ProjectEntry>>,
) -> Option<ProjectEntry> {
    let name = selected.read().clone()?;
    projects.read().iter().find(|p| p.name == name).cloned()
}

#[component]
pub fn ConversationPane() -> Element {
    let selected = use_context::<Signal<Option<String>>>();
    let projects = use_context::<Signal<Vec<ProjectEntry>>>();
    let messages = use_context::<Signal<Vec<ChatMessage>>>();

    let Some(proj) = resolve_selected_project(&selected, &projects) else {
        return rsx! {
            div { class: "chat-area",
                div { class: "chat-empty", "Select a project" }
            }
        };
    };

    let msgs = messages.read().clone();
    let cumulative = sum_token_usage(&msgs);

    rsx! {
        div { class: "chat-area",
            SessionHeader {
                name: proj.name.clone(),
                session_id: proj.session_id.clone(),
                validations: proj.validations,
            }
            if let Some(usage) = cumulative {
                CumulativeTokenBar { usage }
            }
            MessageList { msgs }
        }
    }
}
