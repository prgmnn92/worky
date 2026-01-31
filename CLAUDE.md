# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

worky is a Rust CLI tool for managing work items locally with filesystem storage. It supports syncing with external boards (Azure DevOps, Jira) in future phases. The tool includes an MCP server for Claude Code integration and a web-based kanban board viewer.

## Build & Development Commands

```bash
# Build
cargo build --release

# Run tests
cargo test --workspace

# Run a single test
cargo test --package worky-core test_name

# Install locally
cargo install --path crates/worky-cli --force

# Lint (strict clippy with pedantic + nursery)
cargo clippy --workspace

# Format
cargo fmt --all
```

## Architecture

### Crate Structure

```
crates/
├── worky-core     # Domain models: WorkItem, WorkEvent, patch operations
├── worky-fs       # Filesystem backend: Workspace, storage, config
├── worky-cli      # CLI binary with commands, MCP server, board viewer
└── worky-toolserver  # HTTP API server for AI tool integration
```

### Data Flow

1. **worky-core** defines the domain model (WorkItem, WorkEvent, EventPayload variants) and patch mechanics (SetOperation, JSON merge patch)
2. **worky-fs** implements the Workspace abstraction that persists items as directories (`meta.yml` + `events.ndjson`)
3. **worky-cli** provides the user interface: CLI commands, MCP server (`mcp serve`), and kanban board (`board`)
4. **worky-toolserver** exposes an HTTP API for external AI tools

### Key Design Patterns

- **Append-only event log**: All changes are recorded as events in `events.ndjson` for full history
- **Folder-per-item storage**: Each work item is a directory containing `meta.yml`, `events.ndjson`, `notes.md`
- **UID format**: `fs:<slug>` where slug is derived from title (e.g., `fs:implement-auth`)
- **State workflow**: `TODO → IN_PROGRESS → IN_REVIEW → DONE` (used by `advance`/`revert` commands)

### EventPayload Deserialization

The `EventPayload` enum uses `#[serde(untagged)]` with separate structs that have `#[serde(deny_unknown_fields)]` to prevent ambiguous deserialization. This is critical for correct payload type matching.

## CLI Commands

```bash
worky init                    # Initialize workspace
worky new "Title" [options]   # Create work item
worky add                     # Create interactively
worky list [--state] [--label] [--assignee]
worky get <uid> [--comments N]
worky set <uid> key=value...  # e.g., state=DONE assignee=alice
worky advance <uid>           # Move to next state
worky revert <uid>            # Move to previous state
worky log <uid> -m "message"  # Add comment
worky events <uid>            # Show history
worky board [--port 8080]     # Start kanban web viewer
worky mcp serve               # Start MCP server for Claude Code
```

## MCP Server Integration

The MCP server (`worky mcp serve`) exposes tools: `worky_list`, `worky_get`, `worky_create`, `worky_set`, `worky_log`, `worky_events`, `worky_advance`, `worky_revert`.

Configure in `~/.claude/settings.json`:
```json
{
  "mcpServers": {
    "worky": {
      "command": "worky",
      "args": ["-C", "/path/to/workspace", "mcp", "serve"]
    }
  }
}
```

## Workspace Structure

```
project/
  .worky/config.yml           # Workspace configuration
  work/items/<slug>/
    meta.yml                  # Item metadata (uid, title, state, etc.)
    events.ndjson             # Append-only event log
    notes.md                  # Free-form notes
```
