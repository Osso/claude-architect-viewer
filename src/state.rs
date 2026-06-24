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
    dirs::data_dir().map(|d| {
        d.join("claude-architect")
            .join("logs")
            .join(format!("{name}.jsonl"))
    })
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
        cache_read: usage
            .get("cache_read")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        cache_creation: usage
            .get("cache_creation")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
    })
}

fn parse_line(line: &str) -> Option<ChatMessage> {
    let val: Value = serde_json::from_str(line).ok()?;
    let msg_type = val.get("type")?.as_str()?;
    let timestamp = val
        .get("timestamp")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    match msg_type {
        "user" => {
            let text = val.get("text")?.as_str()?.to_string();
            Some(ChatMessage::User { text, timestamp })
        }
        "assistant" => {
            let text = val.get("text")?.as_str()?.to_string();
            let usage = val.get("usage").and_then(parse_token_usage);
            Some(ChatMessage::Assistant {
                text,
                timestamp,
                usage,
            })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn write_log(name: &str, content: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "claude_architect_viewer_state_{name}_{}.jsonl",
            std::process::id()
        ));
        std::fs::write(&path, content).expect("write test log");
        path
    }

    fn temp_data_home(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "claude_architect_viewer_data_{name}_{}",
            std::process::id()
        ))
    }

    #[test]
    fn parses_user_and_assistant_messages_with_usage() {
        let path = write_log(
            "messages",
            r#"{"type":"user","text":"validate this","timestamp":"2026-01-01T00:00:00Z"}
{"type":"assistant","text":"approved","timestamp":"2026-01-01T00:00:01Z","usage":{"input":11,"output":22,"cache_read":5,"cache_creation":6}}
"#,
        );

        let (messages, offset, had_reset) = parse_jsonl_from_offset(&path, 0);

        assert_eq!(offset, std::fs::metadata(&path).expect("metadata").len());
        assert!(!had_reset);
        assert_eq!(
            messages,
            vec![
                ChatMessage::User {
                    text: "validate this".to_string(),
                    timestamp: "2026-01-01T00:00:00Z".to_string(),
                },
                ChatMessage::Assistant {
                    text: "approved".to_string(),
                    timestamp: "2026-01-01T00:00:01Z".to_string(),
                    usage: Some(TokenUsage {
                        input: 11,
                        output: 22,
                        cache_read: 5,
                        cache_creation: 6,
                    }),
                },
            ]
        );

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn session_reset_discards_prior_messages() {
        let path = write_log(
            "reset",
            r#"{"type":"assistant","text":"old","timestamp":"t1"}
{"type":"session_reset"}
{"type":"user","text":"new","timestamp":"t2"}
"#,
        );

        let (messages, _, had_reset) = parse_jsonl_from_offset(&path, 0);

        assert!(had_reset);
        assert_eq!(
            messages,
            vec![ChatMessage::User {
                text: "new".to_string(),
                timestamp: "t2".to_string(),
            }]
        );

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn offset_reads_incremental_messages_and_missing_file_keeps_offset() {
        let path = write_log(
            "offset",
            r#"{"type":"assistant","text":"old","timestamp":"t1"}
{"type":"user","text":"new","timestamp":"t2"}
"#,
        );
        let first_line_len = r#"{"type":"assistant","text":"old","timestamp":"t1"}
"#
        .len() as u64;

        let (messages, offset, had_reset) = parse_jsonl_from_offset(&path, first_line_len);
        let missing = path.with_extension("missing");
        let (missing_messages, missing_offset, missing_reset) =
            parse_jsonl_from_offset(&missing, 77);

        assert!(!had_reset);
        assert_eq!(offset, std::fs::metadata(&path).expect("metadata").len());
        assert_eq!(
            messages,
            vec![ChatMessage::User {
                text: "new".to_string(),
                timestamp: "t2".to_string(),
            }]
        );
        assert!(missing_messages.is_empty());
        assert_eq!(missing_offset, 77);
        assert!(!missing_reset);

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn load_sessions_reads_sorted_session_entries() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let data_home = temp_data_home("sessions");
        let root = data_home.join("claude-architect");
        std::fs::create_dir_all(root.join("logs")).expect("create logs");
        std::fs::write(
            root.join("sessions.json"),
            r#"{
                "zeta": {"session_id": "session-z", "validations": 2},
                "alpha": {"session_id": "session-a", "validations": 5},
                "broken": {"validations": 9}
            }"#,
        )
        .expect("write sessions");

        let old_data_home = std::env::var_os("XDG_DATA_HOME");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", &data_home);
        }

        let sessions = load_sessions();

        match old_data_home {
            Some(value) => unsafe {
                std::env::set_var("XDG_DATA_HOME", value);
            },
            None => unsafe {
                std::env::remove_var("XDG_DATA_HOME");
            },
        }
        std::fs::remove_dir_all(data_home).ok();

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].name, "alpha");
        assert_eq!(sessions[0].session_id, "session-a");
        assert_eq!(sessions[0].validations, 5);
        assert!(
            sessions[0]
                .jsonl_path
                .as_ref()
                .expect("jsonl path")
                .ends_with("claude-architect/logs/alpha.jsonl")
        );
        assert_eq!(sessions[1].name, "zeta");
    }

    #[test]
    fn parser_ignores_invalid_lines_and_defaults_optional_usage_fields() {
        let path = write_log(
            "invalid",
            r#"invalid
{"type":"assistant","text":"partial","timestamp":"t1","usage":{"input":13,"output":21}}
{"type":"assistant","timestamp":"missing text"}
{"type":"other","text":"ignored","timestamp":"t2"}

"#,
        );

        let (messages, _, had_reset) = parse_jsonl_from_offset(&path, 0);

        assert!(!had_reset);
        assert_eq!(
            messages,
            vec![ChatMessage::Assistant {
                text: "partial".to_string(),
                timestamp: "t1".to_string(),
                usage: Some(TokenUsage {
                    input: 13,
                    output: 21,
                    cache_read: 0,
                    cache_creation: 0,
                }),
            }]
        );

        std::fs::remove_file(path).ok();
    }
}
