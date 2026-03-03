use dioxus::prelude::*;
use crate::state::{ChatMessage, TokenUsage};

const TRUNCATE_THRESHOLD: usize = 500;

#[component]
fn TokenBar(usage: TokenUsage) -> Element {
    rsx! {
        div { class: "token-bar",
            span { class: "token-badge token-input", "in: {usage.input}" }
            span { class: "token-badge token-output", "out: {usage.output}" }
            span { class: "token-badge token-cache-read", "cache_r: {usage.cache_read}" }
            span { class: "token-badge token-cache-write", "cache_w: {usage.cache_creation}" }
        }
    }
}

#[component]
fn UserMessage(text: String, timestamp: String) -> Element {
    let mut expanded = use_signal(|| false);
    let is_long = text.len() > TRUNCATE_THRESHOLD;

    let display_text = if is_long && !*expanded.read() {
        format!("{}…", &text[..TRUNCATE_THRESHOLD])
    } else {
        text.clone()
    };

    rsx! {
        div { class: "message message-user",
            div { class: "message-label", "User" }
            div {
                class: if is_long { "message-text message-expandable" } else { "message-text" },
                "{display_text}"
            }
            if is_long {
                button {
                    class: "message-expand-btn",
                    onclick: move |_| {
                        let cur = *expanded.read();
                        expanded.set(!cur);
                    },
                    if *expanded.read() { "show less" } else { "show more" }
                }
            }
        }
    }
}

#[component]
fn AssistantMessage(text: String, timestamp: String, usage: Option<TokenUsage>) -> Element {
    let mut expanded = use_signal(|| false);
    let is_long = text.len() > TRUNCATE_THRESHOLD;

    let display_text = if is_long && !*expanded.read() {
        format!("{}…", &text[..TRUNCATE_THRESHOLD])
    } else {
        text.clone()
    };

    rsx! {
        div { class: "message message-assistant",
            div { class: "message-label", "Assistant" }
            div {
                class: if is_long { "message-text message-expandable" } else { "message-text" },
                "{display_text}"
            }
            if is_long {
                button {
                    class: "message-expand-btn",
                    onclick: move |_| {
                        let cur = *expanded.read();
                        expanded.set(!cur);
                    },
                    if *expanded.read() { "show less" } else { "show more" }
                }
            }
            if let Some(u) = usage {
                TokenBar { usage: u }
            }
        }
    }
}

#[component]
pub fn MessageRow(msg: ChatMessage) -> Element {
    match msg {
        ChatMessage::User { text, timestamp } => {
            rsx! { UserMessage { text, timestamp } }
        }
        ChatMessage::Assistant { text, timestamp, usage } => {
            rsx! { AssistantMessage { text, timestamp, usage } }
        }
    }
}
