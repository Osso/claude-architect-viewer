use serde_json::Value;
use std::collections::HashMap;
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

fn log_path_for_project(name: &str) -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("claude-architect").join("logs").join(format!("{name}.jsonl")))
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
            let jsonl_path = log_path_for_project(&name);
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

fn parse_token_usage(usage: &Value) -> Option<TokenUsage> {
    Some(TokenUsage {
        input: usage.get("input")?.as_u64().unwrap_or(0),
        output: usage.get("output")?.as_u64().unwrap_or(0),
        cache_read: usage.get("cache_read").and_then(|v| v.as_u64()).unwrap_or(0),
        cache_creation: usage.get("cache_creation").and_then(|v| v.as_u64()).unwrap_or(0),
    })
}

fn parse_line(line: &str) -> Option<ChatMessage> {
    let val: Value = serde_json::from_str(line).ok()?;
    let msg_type = val.get("type")?.as_str()?;
    let timestamp = val.get("timestamp").and_then(|t| t.as_str()).unwrap_or("").to_string();

    match msg_type {
        "user" => {
            let text = val.get("text")?.as_str()?.to_string();
            Some(ChatMessage::User { text, timestamp })
        }
        "assistant" => {
            let text = val.get("text")?.as_str()?.to_string();
            let usage = val.get("usage").and_then(parse_token_usage);
            Some(ChatMessage::Assistant { text, timestamp, usage })
        }
        _ => None,
    }
}

pub fn parse_jsonl_from_offset(path: &Path, offset: u64) -> (Vec<ChatMessage>, u64, bool) {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return (vec![], offset, false),
    };

    if file.seek(SeekFrom::Start(offset)).is_err() {
        return (vec![], offset, false);
    }

    let mut reader = BufReader::new(&mut file);
    let mut messages = Vec::new();
    let mut current_offset = offset;
    let mut had_reset = false;

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

        if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
            if val.get("type").and_then(|t| t.as_str()) == Some("session_reset") {
                messages.clear();
                had_reset = true;
                continue;
            }
        }

        if let Some(msg) = parse_line(trimmed) {
            messages.push(msg);
        }
    }

    (messages, current_offset, had_reset)
}
