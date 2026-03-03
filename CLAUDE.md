# claude-architect-viewer

Dioxus 0.6 desktop app for viewing claude-architect conversations in real-time.

## Architecture

- `src/state.rs` — Types (ProjectEntry, ChatMessage, TokenUsage) + JSONL parsing
- `src/watcher.rs` — notify crate file watcher for sessions.json + JSONL dirs
- `src/sidebar.rs` — Project list with validation counts
- `src/message.rs` — Message bubble + per-message token bar
- `src/chat.rs` — Message feed + cumulative token bar + session header
- `src/main.rs` — Dioxus desktop launch + watcher thread

## Data Sources

- `~/.local/share/claude-architect/sessions.json` — project list + session IDs
- `~/.claude/projects/{encoded-path}/{session-id}.jsonl` — conversation messages
- `~/.local/share/claude-architect/designs/{project}.md` — design docs

## Build & Run

```bash
cargo run
```
