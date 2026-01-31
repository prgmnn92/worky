# worky

A CLI tool for managing work items locally with filesystem storage. Includes an MCP server for Claude Code integration and a web-based kanban board viewer.

## Features

- **Local-first**: Work items stored as files in your project directory
- **Append-only event log**: Full history of all changes
- **Claude Code integration**: MCP server exposes work item tools
- **Kanban board**: Web-based visual board viewer
- **Flexible workflow**: `TODO → IN_PROGRESS → IN_REVIEW → DONE`

## Installation

```bash
cargo install --path crates/worky-cli
```

## Quick Start

```bash
# Initialize a workspace
worky init

# Create work items
worky new "Implement user authentication" -l backend -l security
worky add  # Interactive mode

# List and manage
worky list
worky get fs:implement-user-authentication
worky set fs:implement-user-authentication state=IN_PROGRESS assignee=alice
worky advance fs:implement-user-authentication  # Move to next state
worky log fs:implement-user-authentication -m "Started working on OAuth flow"

# View kanban board
worky board --port 8080
# Open http://127.0.0.1:8080
```

## Commands

| Command | Description |
|---------|-------------|
| `worky init` | Initialize workspace in current directory |
| `worky new "title"` | Create new work item |
| `worky add` | Create work item interactively |
| `worky list` | List all work items |
| `worky get <uid>` | Show work item details |
| `worky set <uid> key=value` | Update work item fields |
| `worky advance <uid>` | Move to next state |
| `worky revert <uid>` | Move to previous state |
| `worky log <uid> -m "msg"` | Add comment |
| `worky events <uid>` | Show change history |
| `worky board` | Start kanban board web viewer |
| `worky mcp serve` | Start MCP server for Claude Code |

## Claude Code Integration

Configure in `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "worky": {
      "command": "worky",
      "args": ["-C", "/path/to/your/workspace", "mcp", "serve"]
    }
  }
}
```

Available MCP tools: `worky_list`, `worky_get`, `worky_create`, `worky_set`, `worky_log`, `worky_events`, `worky_advance`, `worky_revert`

## Workspace Structure

```
project/
  .worky/config.yml           # Workspace configuration
  work/items/<slug>/
    meta.yml                  # Item metadata
    events.ndjson             # Append-only event log
    notes.md                  # Free-form notes
```

## License

MIT
