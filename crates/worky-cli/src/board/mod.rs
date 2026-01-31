//! Kanban board web viewer for work items.

mod html;

use anyhow::{Context, Result};
use axum::{
    extract::State,
    http::header,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;
use worky_fs::Workspace;

struct AppState {
    workspace_path: PathBuf,
}

/// Serve the kanban board on the specified host and port.
pub fn serve(workspace_path: &std::path::Path, host: &str, port: u16) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        serve_async(workspace_path, host, port).await
    })
}

async fn serve_async(workspace_path: &std::path::Path, host: &str, port: u16) -> Result<()> {
    // Verify workspace exists
    let _ = Workspace::open(workspace_path).context("Failed to open workspace")?;

    let state = Arc::new(AppState {
        workspace_path: workspace_path.to_path_buf(),
    });

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/items", get(items_handler))
        .route("/styles.css", get(styles_handler))
        .with_state(state);

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = TcpListener::bind(addr).await?;

    info!("Kanban board available at http://{}", addr);
    println!("🎯 Kanban board running at http://{addr}");
    println!("   Press Ctrl+C to stop");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn index_handler() -> Html<&'static str> {
    Html(html::INDEX_HTML)
}

async fn styles_handler() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css")],
        html::STYLES_CSS,
    )
}

async fn items_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let ws = match Workspace::open(&state.workspace_path) {
        Ok(ws) => ws,
        Err(e) => {
            return Json(serde_json::json!({
                "error": format!("Failed to open workspace: {e}")
            }));
        }
    };

    let items = match ws.list_items(None) {
        Ok(items) => items,
        Err(e) => {
            return Json(serde_json::json!({
                "error": format!("Failed to list items: {e}")
            }));
        }
    };

    // Group items by state and include comments
    let items_with_comments: Vec<serde_json::Value> = items
        .iter()
        .map(|item| {
            let comments = ws
                .read_events(&item.uid, None)
                .unwrap_or_default()
                .into_iter()
                .filter(|e| matches!(e.event_type, worky_core::EventType::CommentAdded))
                .map(|e| {
                    let message = if let worky_core::EventPayload::Comment(p) = &e.payload {
                        p.message.clone()
                    } else {
                        String::new()
                    };
                    serde_json::json!({
                        "timestamp": e.timestamp.format("%Y-%m-%d %H:%M").to_string(),
                        "actor": e.actor,
                        "message": message
                    })
                })
                .collect::<Vec<_>>();

            serde_json::json!({
                "uid": item.uid,
                "title": item.title,
                "state": item.state,
                "assignee": item.assignee,
                "labels": item.labels,
                "created_at": item.created_at.format("%Y-%m-%d %H:%M").to_string(),
                "updated_at": item.updated_at.format("%Y-%m-%d %H:%M").to_string(),
                "fields": item.fields,
                "comments": comments
            })
        })
        .collect();

    Json(serde_json::json!({ "items": items_with_comments }))
}
