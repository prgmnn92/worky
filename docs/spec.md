# worky Specification v0.1

## Overview

worky is a CLI tool for managing work items locally and syncing with external boards (Azure DevOps, Jira, etc.).

## Workspace Convention

### Directory Structure

```
project/
  .worky/
    config.yml          # Workspace configuration
    index.sqlite        # Optional search index (Phase 2+)
  work/
    items/
      <slug>/
        meta.yml        # Work item metadata
        events.ndjson   # Append-only event log
        notes.md        # Free-form notes
        links.md        # Related links/references
        artifacts/      # Attached files
```

### Configuration (.worky/config.yml)

```yaml
version: 1
workspace:
  name: "my-project"

defaults:
  state: "TODO"
  labels: []

backends:
  - type: filesystem
    path: "./work/items"

# Future: board connectors
# connectors:
#   - type: azure-devops
#     org: myorg
#     project: myproject
```

## Work Item Schema

### Normalized Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| uid | string | yes | Unique identifier (e.g., `fs:implement-auth`) |
| title | string | yes | Human-readable title |
| state | string | yes | Current state (TODO, IN_PROGRESS, DONE, etc.) |
| assignee | string | no | Assigned person |
| labels | string[] | no | Categorization labels |
| created_at | datetime | yes | ISO 8601 UTC timestamp |
| updated_at | datetime | yes | ISO 8601 UTC timestamp |

### Custom Fields

Custom fields are stored under `fields` as a nested map:

```yaml
fields:
  priority: high
  estimate_hours: 8
  System:
    IterationPath: "Sprint 1"
    AreaPath: "Backend"
```

### meta.yml Example

```yaml
uid: "fs:implement-auth-redirect"
title: "Implement OAuth redirect handler"
state: "IN_PROGRESS"
assignee: "alice"
labels:
  - backend
  - security
created_at: "2025-01-31T10:00:00Z"
updated_at: "2025-01-31T14:30:00Z"
fields:
  priority: high
  estimate_hours: 4
```

## Event Schema

Events are stored in NDJSON format (one JSON object per line).

### Event Types

| Type | Description |
|------|-------------|
| CREATED | Item was created |
| STATE_CHANGED | State transition |
| FIELD_CHANGED | Field value updated |
| COMMENT_ADDED | Comment/note added |
| LABEL_ADDED | Label attached |
| LABEL_REMOVED | Label removed |
| ASSIGNED | Assignee changed |
| AI_ACTION | Action performed by AI tool |

### Event Structure

```json
{
  "id": "evt_abc123",
  "type": "STATE_CHANGED",
  "timestamp": "2025-01-31T14:30:00Z",
  "actor": "alice",
  "payload": {
    "from": "TODO",
    "to": "IN_PROGRESS"
  }
}
```

## UID Format

### Filesystem Backend

- Format: `fs:<slug>`
- Slug: lowercase, hyphenated (generated from title)
- Example: `fs:implement-auth-redirect`

### Future Backends

- Azure DevOps: `ado:<org>/<project>/<id>`
- Jira: `jira:<project>/<key>`

## Patch Mechanics

### Set Operations

Path-based field assignment:

```bash
worky set <uid> state=IN_PROGRESS assignee=alice
worky set <uid> fields.priority=high
worky set <uid> fields.System.IterationPath="Sprint 2"
```

Path resolution:
- `state` → `/state`
- `fields.priority` → `/fields/priority`
- `fields.System.IterationPath` → `/fields/System/IterationPath`

### Merge Patch

JSON Merge Patch (RFC 7396) for complex updates:

```bash
worky patch <uid> --merge '{"fields": {"priority": "critical", "blocked": true}}'
```

## CLI Commands (MVP)

```
worky init [--path .]              # Initialize workspace
worky new "Title" [options]        # Create work item
worky list [--state] [--label]     # List items
worky get <uid>                    # Show item details
worky set <uid> key=value...       # Set field values
worky patch <uid> --merge <json>   # Apply merge patch
worky events <uid> [--since 7d]    # Show event history
worky log <uid> <TYPE> -m "msg"    # Add manual event
```

## Tool Server API (Phase 2)

Local HTTP server for AI tool integration.

### Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | /health | Health check |
| POST | /search | Search items |
| GET | /items/:uid | Get item |
| POST | /items/:uid/set | Set fields |
| POST | /items/:uid/events | Append event |

### Security

- Binds to `127.0.0.1` only (no network exposure)
- Configurable field allowlist for writes
- All AI actions logged with `AI_ACTION` event type
