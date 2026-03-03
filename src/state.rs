use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct ProjectEntry {
    pub name: String,
    pub session_id: String,
    pub validations: u32,
    pub jsonl_path: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChatMessage {
    User {
        text: String,
        timestamp: String,
    },
    Assistant {
        text: String,
        timestamp: String,
        usage: Option<TokenUsage>,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_creation: u64,
}

fn jsonl_path_for_session(session_id: &str) -> Option<PathBuf> {
    let projects_dir = dirs::home_dir()?.join(".claude").join("projects");
    let entries = std::fs::read_dir(&projects_dir).ok()?;

    for entry in entries.flatten() {
        if !entry.file_type().ok()?.is_dir() {
            continue;
        }
        let candidate = entry.path().join(format!("{}.jsonl", session_id));
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

pub fn load_sessions() -> Vec<ProjectEntry> {
    let sessions_path = match dirs::data_dir() {
        Some(d) => d.join("claude-architect").join("sessions.json"),
        None => return vec![],
    };

    let content = match std::fs::read_to_string(&sessions_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let map: HashMap<String, Value> = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(_) => return vec![],
    };

    let mut entries: Vec<ProjectEntry> = map
        .into_iter()
        .filter_map(|(name, val)| {
            let session_id = val.get("session_id")?.as_str()?.to_string();
            let validations = val.get("validations")?.as_u64().unwrap_or(0) as u32;
            let jsonl_path = jsonl_path_for_session(&session_id);
            Some(ProjectEntry {
                name,
                session_id,
                validations,
                jsonl_path,
            })
        })
        .collect();

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

const SKIP_TYPES: &[&str] = &[
    "progress",
    "queue-operation",
    "system",
    "file-history-snapshot",
];

fn parse_token_usage(usage: &Value) -> Option<TokenUsage> {
    Some(TokenUsage {
        input: usage.get("input_tokens")?.as_u64().unwrap_or(0),
        output: usage.get("output_tokens")?.as_u64().unwrap_or(0),
        cache_read: usage
            .get("cache_read_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        cache_creation: usage
            .get("cache_creation_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
    })
}

fn parse_line(line: &str, seen_uuids: &mut HashSet<String>) -> Option<ChatMessage> {
    let val: Value = serde_json::from_str(line).ok()?;

    let msg_type = val.get("type")?.as_str()?;
    if SKIP_TYPES.contains(&msg_type) {
        return None;
    }

    let uuid = val.get("uuid").and_then(|u| u.as_str()).unwrap_or("").to_string();
    if !uuid.is_empty() {
        if seen_uuids.contains(&uuid) {
            return None;
        }
        seen_uuids.insert(uuid);
    }

    let timestamp = val
        .get("timestamp")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    match msg_type {
        "user" => parse_user_message(&val, timestamp),
        "assistant" => parse_assistant_message(&val, timestamp),
        _ => None,
    }
}

fn parse_user_message(val: &Value, timestamp: String) -> Option<ChatMessage> {
    let content = val.get("message")?.get("content")?;

    let text = match content {
        Value::String(s) => s.clone(),
        Value::Array(_) => return None, // tool results, skip
        _ => return None,
    };

    if text.trim().is_empty() {
        return None;
    }

    Some(ChatMessage::User { text, timestamp })
}

fn parse_assistant_message(val: &Value, timestamp: String) -> Option<ChatMessage> {
    let message = val.get("message")?;
    let content = message.get("content")?.as_array()?;

    let text = extract_text_blocks(content)?;
    let usage = message.get("usage").and_then(|u| parse_token_usage(u));

    Some(ChatMessage::Assistant {
        text,
        timestamp,
        usage,
    })
}

fn extract_text_blocks(content: &[Value]) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();

    for block in content {
        if block.get("type").and_then(|t| t.as_str()) == Some("text") {
            if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                parts.push(text);
            }
        }
    }

    if parts.is_empty() {
        return None;
    }

    Some(parts.join("\n"))
}

pub fn parse_jsonl(path: &Path) -> Vec<ChatMessage> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return vec![],
    };

    let reader = BufReader::new(file);
    let mut seen_uuids = HashSet::new();
    let mut messages = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(msg) = parse_line(trimmed, &mut seen_uuids) {
            messages.push(msg);
        }
    }

    messages
}

pub fn parse_jsonl_from_offset(path: &Path, offset: u64) -> (Vec<ChatMessage>, u64) {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return (vec![], offset),
    };

    if file.seek(SeekFrom::Start(offset)).is_err() {
        return (vec![], offset);
    }

    let mut reader = BufReader::new(&mut file);
    let mut seen_uuids = HashSet::new();
    let mut messages = Vec::new();
    let mut current_offset = offset;

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(n) => current_offset += n as u64,
            Err(_) => break,
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(msg) = parse_line(trimmed, &mut seen_uuids) {
            messages.push(msg);
        }
    }

    (messages, current_offset)
}
