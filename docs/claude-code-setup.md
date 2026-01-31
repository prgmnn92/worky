# Claude Code Integration with worky

This guide explains how to configure Claude Code to use worky for work item management.

## Prerequisites

1. Install worky:
   ```bash
   cargo install --path crates/worky-cli
   ```

2. Initialize a workspace:
   ```bash
   mkdir ~/my-project
   cd ~/my-project
   worky init
   ```

## Configuration

Add the worky MCP server to your Claude Code settings.

### Option 1: Project-specific configuration

Create or edit `.claude/settings.json` in your project directory:

```json
{
  "mcpServers": {
    "worky": {
      "command": "worky",
      "args": ["mcp", "serve"],
      "cwd": "/path/to/your/workspace"
    }
  }
}
```

### Option 2: Global configuration

Edit `~/.claude/settings.json`:

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

### Option 3: Using environment variable

```json
{
  "mcpServers": {
    "worky": {
      "command": "worky",
      "args": ["-C", "${WORKCTL_PATH}", "mcp", "serve"],
      "env": {
        "WORKCTL_PATH": "/path/to/your/workspace"
      }
    }
  }
}
```

## Available Tools

Once configured, Claude Code will have access to these tools:

### `worky_list`
List work items with optional filtering.

**Parameters:**
- `state` (optional): Filter by state (e.g., "TODO", "IN_PROGRESS", "DONE")
- `assignee` (optional): Filter by assignee name
- `label` (optional): Filter by label

**Example:** "Show me all TODO items assigned to alice"

### `worky_get`
Get detailed information about a specific work item.

**Parameters:**
- `uid` (required): The work item UID (e.g., "fs:implement-auth")
- `comments` (optional, default: 10): Number of recent comments to include

**Example:** "Get the details of fs:implement-auth"

### `worky_create`
Create a new work item.

**Parameters:**
- `title` (required): Title of the work item
- `state` (optional): Initial state (default: TODO)
- `assignee` (optional): Assignee for the item
- `labels` (optional): Array of labels
- `description` (optional): Description of the work item

**Example:** "Create a new task for implementing user authentication"

### `worky_set`
Update fields on a work item.

**Parameters:**
- `uid` (required): The work item UID
- `state` (optional): New state value
- `assignee` (optional): New assignee (empty string to unassign)
- `labels` (optional): Replace all labels
- `fields` (optional): Custom fields to set

**Example:** "Mark fs:implement-auth as IN_PROGRESS and assign to bob"

### `worky_log`
Add a comment or note to a work item.

**Parameters:**
- `uid` (required): The work item UID
- `message` (required): The comment/note to add

**Example:** "Add a note to fs:implement-auth that the API is ready for review"

### `worky_events`
Get the event history for a work item.

**Parameters:**
- `uid` (required): The work item UID
- `since_days` (optional): Only show events from the last N days

**Example:** "Show me the history of changes to fs:implement-auth"

## Usage Examples

Once configured, you can ask Claude Code things like:

- "Create a task for implementing the login page with labels frontend and auth"
- "What tasks are currently in progress?"
- "Show me the details of the authentication task"
- "Mark the login task as done"
- "Add a note that we decided to use JWT for authentication"
- "What changes were made to the auth task in the last 7 days?"

## Troubleshooting

### MCP server not connecting

1. Verify worky is in your PATH:
   ```bash
   which worky
   ```

2. Test the MCP server manually:
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | worky -C /path/to/workspace mcp serve
   ```

3. Check that the workspace is initialized:
   ```bash
   ls /path/to/workspace/.worky/config.yml
   ```

### Tool calls failing

Check the workspace path in your configuration. The path must point to a directory containing an initialized worky workspace (has `.worky/config.yml`).
