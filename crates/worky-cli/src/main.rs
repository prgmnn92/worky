//! worky CLI - Work item management from the command line.

mod board;
mod commands;
mod interactive;
mod mcp;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser)]
#[command(name = "worky")]
#[command(author, version, about = "Work item management CLI")]
#[command(propagate_version = true)]
struct Cli {
    /// Output format
    #[arg(long, global = true, default_value = "human")]
    format: output::OutputFormat,

    /// Workspace path (defaults to current directory)
    #[arg(long, short = 'C', global = true)]
    path: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new workspace
    Init,

    /// Create a new work item
    New {
        /// Title of the work item (omit for interactive mode)
        title: Option<String>,

        /// Interactive mode - prompt for all fields
        #[arg(long, short = 'i')]
        interactive: bool,

        /// Initial state
        #[arg(long, short = 's')]
        state: Option<String>,

        /// Labels (can be specified multiple times)
        #[arg(long, short = 'l')]
        label: Vec<String>,

        /// Assignee
        #[arg(long, short = 'a')]
        assignee: Option<String>,

        /// Description
        #[arg(long, short = 'd')]
        description: Option<String>,
    },

    /// Create a new work item interactively (alias for `new -i`)
    Add,

    /// List work items
    #[command(alias = "ls")]
    List {
        /// Filter by state
        #[arg(long, short = 's')]
        state: Option<String>,

        /// Filter by assignee
        #[arg(long, short = 'a')]
        assignee: Option<String>,

        /// Filter by label
        #[arg(long, short = 'l')]
        label: Option<String>,
    },

    /// Get a work item by UID
    Get {
        /// Work item UID (e.g., fs:implement-auth)
        uid: String,

        /// Show recent comments (default: 5, use 0 to hide)
        #[arg(long, short = 'c', default_value = "5")]
        comments: usize,
    },

    /// Set field values on a work item
    Set {
        /// Work item UID
        uid: String,

        /// Field assignments (key=value)
        #[arg(required = true)]
        assignments: Vec<String>,
    },

    /// Apply a JSON merge patch to a work item
    Patch {
        /// Work item UID
        uid: String,

        /// JSON merge patch
        #[arg(long)]
        merge: String,
    },

    /// Show event history for a work item
    Events {
        /// Work item UID
        uid: String,

        /// Show events from the last N days
        #[arg(long)]
        since: Option<u32>,
    },

    /// Add a comment/log entry to a work item
    Log {
        /// Work item UID
        uid: String,

        /// Comment message
        #[arg(short = 'm', long)]
        message: String,
    },

    /// Advance a work item to the next state in the workflow
    #[command(alias = "next")]
    Advance {
        /// Work item UID
        uid: String,
    },

    /// Move a work item back to the previous state
    #[command(alias = "prev")]
    Revert {
        /// Work item UID
        uid: String,
    },

    /// Start the tool server (for AI integration)
    #[command(subcommand)]
    Tool(ToolCommands),

    /// MCP server for Claude Code integration
    #[command(subcommand)]
    Mcp(McpCommands),

    /// Start the kanban board web viewer
    Board {
        /// Port to listen on
        #[arg(long, short = 'p', default_value = "8080")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },
}

#[derive(Subcommand)]
enum ToolCommands {
    /// Start the HTTP tool server
    Serve {
        /// Port to listen on
        #[arg(long, short = 'p', default_value = "17373")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },
}

#[derive(Subcommand)]
enum McpCommands {
    /// Start the MCP server (communicates via stdin/stdout)
    Serve,
}

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    // Determine workspace path
    let workspace_path = cli
        .path
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    match cli.command {
        Commands::Init => commands::init(&workspace_path, cli.format),
        Commands::New {
            title,
            interactive,
            state,
            label,
            assignee,
            description,
        } => {
            // Use interactive mode if flag is set or no title provided
            if interactive || title.is_none() {
                commands::new_interactive(&workspace_path, cli.format)
            } else {
                commands::new_item(
                    &workspace_path,
                    title.as_deref().unwrap_or(""),
                    state,
                    label,
                    assignee,
                    description,
                    cli.format,
                )
            }
        }
        Commands::Add => commands::new_interactive(&workspace_path, cli.format),
        Commands::List {
            state,
            assignee,
            label,
        } => commands::list(&workspace_path, state, assignee, label, cli.format),
        Commands::Get { uid, comments } => {
            commands::get(&workspace_path, &uid, comments, cli.format)
        }
        Commands::Set { uid, assignments } => {
            commands::set(&workspace_path, &uid, &assignments, cli.format)
        }
        Commands::Patch { uid, merge } => {
            commands::patch(&workspace_path, &uid, &merge, cli.format)
        }
        Commands::Events { uid, since } => {
            commands::events(&workspace_path, &uid, since, cli.format)
        }
        Commands::Log { uid, message } => {
            commands::log(&workspace_path, &uid, &message, cli.format)
        }
        Commands::Advance { uid } => commands::advance(&workspace_path, &uid, cli.format),
        Commands::Revert { uid } => commands::revert(&workspace_path, &uid, cli.format),
        Commands::Tool(ToolCommands::Serve { port, host }) => {
            commands::tool_serve(&workspace_path, &host, port)
        }
        Commands::Mcp(McpCommands::Serve) => mcp::serve(&workspace_path),
        Commands::Board { port, host } => board::serve(&workspace_path, &host, port),
    }
}
