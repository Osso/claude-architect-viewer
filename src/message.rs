#![cfg_attr(coverage_nightly, coverage(off))]

use crate::state::{ChatMessage, TokenUsage};
use dioxus::prelude::*;

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
    rsx! {
        ExpandableMessage {
            role: "User",
            text,
            timestamp,
            usage: None,
            class_name: "message message-user",
        }
    }
}

#[component]
fn AssistantMessage(text: String, timestamp: String, usage: Option<TokenUsage>) -> Element {
    rsx! {
        ExpandableMessage {
            role: "Assistant",
            text,
            timestamp,
            usage,
            class_name: "message message-assistant",
        }
    }
}

#[component]
fn ExpandableMessage(
    role: &'static str,
    text: String,
    timestamp: String,
    usage: Option<TokenUsage>,
    class_name: &'static str,
) -> Element {
    let expanded = use_signal(|| false);
    let is_long = text.len() > TRUNCATE_THRESHOLD;
    let is_expanded = *expanded.read();
    let display_text = truncated_text(&text, is_long, is_expanded);
    let time = format_time(&timestamp);
    let text_class = text_class_name(is_long);
    let button_label = expand_button_label(is_expanded);

    rsx! {
        div { class: "{class_name}",
            MessageLabel { role, time }
            MessageBody { text_class, display_text }
            ExpandButton { is_long, expanded, button_label }
            UsageBar { usage }
        }
    }
}

fn truncated_text(text: &str, is_long: bool, is_expanded: bool) -> String {
    if is_long && !is_expanded {
        return format!("{}…", &text[..TRUNCATE_THRESHOLD]);
    }
    text.to_string()
}

fn text_class_name(is_long: bool) -> &'static str {
    if is_long {
        "message-text message-expandable"
    } else {
        "message-text"
    }
}

fn expand_button_label(is_expanded: bool) -> &'static str {
    if is_expanded {
        "show less"
    } else {
        "show more"
    }
}

#[component]
fn MessageLabel(role: &'static str, time: String) -> Element {
    rsx! {
        div { class: "message-label",
            span { "{role}" }
            span { class: "message-time", "{time}" }
        }
    }
}

#[component]
fn MessageBody(text_class: &'static str, display_text: String) -> Element {
    rsx! {
        div {
            class: "{text_class}",
            "{display_text}"
        }
    }
}

#[component]
fn ExpandButton(is_long: bool, mut expanded: Signal<bool>, button_label: &'static str) -> Element {
    if !is_long {
        return rsx! {};
    }
    rsx! {
        button {
            class: "message-expand-btn",
            onclick: move |_| {
                let cur = *expanded.read();
                expanded.set(!cur);
            },
            "{button_label}"
        }
    }
}

#[component]
fn UsageBar(usage: Option<TokenUsage>) -> Element {
    let Some(usage) = usage else {
        return rsx! {};
    };
    rsx! { TokenBar { usage } }
}

#[component]
pub fn MessageRow(msg: ChatMessage) -> Element {
    match msg {
        ChatMessage::User { text, timestamp } => {
            rsx! { UserMessage { text, timestamp } }
        }
        ChatMessage::Assistant {
            text,
            timestamp,
            usage,
        } => {
            rsx! { AssistantMessage { text, timestamp, usage } }
        }
    }
}
