use dioxus::prelude::*;
use crate::state::{ChatMessage, TokenUsage};

const TRUNCATE_THRESHOLD: usize = 500;

fn format_time(timestamp: &str) -> String {
    // "2026-02-27T07:06:44.918Z" → "07:06:44"
    timestamp
        .split('T')
        .nth(1)
        .and_then(|t| t.split('.').next())
        .unwrap_or("")
        .to_string()
}

fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

#[component]
fn TokenBar(usage: TokenUsage) -> Element {
    let context = usage.input + usage.cache_read + usage.cache_creation;
    rsx! {
        div { class: "token-bar",
            span { class: "token-badge token-input", "ctx: {format_tokens(context)}" }
            span { class: "token-badge token-output", "out: {format_tokens(usage.output)}" }
            span { class: "token-badge token-cache-read", "cache_r: {format_tokens(usage.cache_read)}" }
            span { class: "token-badge token-cache-write", "cache_w: {format_tokens(usage.cache_creation)}" }
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

    let time = format_time(&timestamp);
    rsx! {
        div { class: "message message-user",
            div { class: "message-label",
                span { "User" }
                span { class: "message-time", "{time}" }
            }
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

    let time = format_time(&timestamp);
    rsx! {
        div { class: "message message-assistant",
            div { class: "message-label",
                span { "Assistant" }
                span { class: "message-time", "{time}" }
            }
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
