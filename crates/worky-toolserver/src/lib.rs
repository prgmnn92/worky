//! HTTP tool server for AI integration.
//!
//! Provides a local HTTP API that Claude and other AI tools can use
//! to interact with worky workspaces.

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;
use worky_core::{SetOperation, WorkEvent, WorkItem};
use worky_fs::{workspace::ItemFilter, Workspace};

/// Server state shared across handlers.
struct AppState {
    workspace_path: PathBuf,
}

/// Start the tool server.
///
/// # Errors
/// Returns error if binding fails or server encounters an error.
pub async fn serve(workspace_path: &std::path::Path, host: &str, port: u16) -> Result<()> {
    let state = Arc::new(AppState {
        workspace_path: workspace_path.to_path_buf(),
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/search", post(search))
        .route("/items/{uid}", get(get_item))
        .route("/items/{uid}/set", post(set_fields))
        .route("/items/{uid}/events", get(get_events).post(add_event))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("{host}:{port}");
    info!(address = %addr, "Starting tool server");

    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// --- Request/Response types ---

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

#[derive(Debug, Deserialize)]
struct SearchRequest {
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    assignee: Option<String>,
    #[serde(default)]
    label: Option<String>,
}

#[derive(Debug, Serialize)]
struct SearchResponse {
    items: Vec<WorkItem>,
    count: usize,
}

#[derive(Debug, Deserialize)]
struct SetFieldsRequest {
    /// Field assignments as key=value pairs
    assignments: Vec<String>,
    /// Optional actor name (for audit logging)
    #[serde(default)]
    actor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AddEventRequest {
    /// Event type
    #[serde(rename = "type")]
    event_type: String,
    /// Event message
    message: String,
    /// Optional actor name
    #[serde(default)]
    actor: Option<String>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

// --- Handlers ---

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn search(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, AppError> {
    let ws = Workspace::open(&state.workspace_path)?;

    let filter = if req.state.is_some() || req.assignee.is_some() || req.label.is_some() {
        Some(ItemFilter {
            state: req.state,
            assignee: req.assignee,
            label: req.label,
        })
    } else {
        None
    };

    let items = ws.list_items(filter.as_ref())?;
    let count = items.len();

    Ok(Json(SearchResponse { items, count }))
}

async fn get_item(
    State(state): State<Arc<AppState>>,
    Path(uid): Path<String>,
) -> Result<Json<WorkItem>, AppError> {
    let ws = Workspace::open(&state.workspace_path)?;
    let item = ws.get_item(&uid)?;
    Ok(Json(item))
}

async fn set_fields(
    State(state): State<Arc<AppState>>,
    Path(uid): Path<String>,
    Json(req): Json<SetFieldsRequest>,
) -> Result<Json<WorkItem>, AppError> {
    let ws = Workspace::open(&state.workspace_path)?;

    // Parse assignments
    let operations: Vec<SetOperation> = req
        .assignments
        .iter()
        .map(|a| SetOperation::parse(a))
        .collect::<worky_core::Result<Vec<_>>>()?;

    // Update item
    let item = ws.update_item(&uid, &operations)?;

    // Log AI action if actor specified
    if let Some(actor) = req.actor {
        let event = WorkEvent::ai_action("worky-toolserver", "set_fields").with_actor(actor);
        let slug = uid.strip_prefix("fs:").unwrap_or(&uid);
        ws.append_event(slug, &event)?;
    }

    Ok(Json(item))
}

async fn get_events(
    State(state): State<Arc<AppState>>,
    Path(uid): Path<String>,
) -> Result<Json<Vec<WorkEvent>>, AppError> {
    let ws = Workspace::open(&state.workspace_path)?;
    let events = ws.read_events(&uid, None)?;
    Ok(Json(events))
}

async fn add_event(
    State(state): State<Arc<AppState>>,
    Path(uid): Path<String>,
    Json(req): Json<AddEventRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ws = Workspace::open(&state.workspace_path)?;

    // Validate item exists
    ws.get_item(&uid)?;

    // Add comment (simplified - full event support would need more logic)
    ws.add_comment(&uid, &req.message)?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "message": "Event added"
    })))
}

// --- Error handling ---

struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = if self.0.to_string().contains("not found") {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };

        let body = Json(ErrorResponse {
            error: self.0.to_string(),
        });

        (status, body).into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
