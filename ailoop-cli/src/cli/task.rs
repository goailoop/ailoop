use clap::{Parser, Subcommand};

#[derive(Subcommand)]
pub enum TaskCommands {
    /// Create a new task
    Create {
        /// Task title
        title: String,

        /// Detailed task description
        #[arg(short, long)]
        description: String,

        /// Channel name (default: public)
        #[arg(short, long, default_value = "public")]
        channel: String,

        /// Server URL for remote operation
        #[arg(long, default_value = "")]
        server: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// List all tasks
    List {
        /// Channel name (default: public)
        #[arg(short, long, default_value = "public")]
        channel: String,

        /// Filter by task state
        #[arg(long)]
        state: Option<String>,

        /// Server URL for remote operation
        #[arg(long, default_value = "")]
        server: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Show task details
    Show {
        /// Task ID
        task_id: String,

        /// Channel name (default: public)
        #[arg(short, long, default_value = "public")]
        channel: String,

        /// Server URL for remote operation
        #[arg(long, default_value = "")]
        server: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Update task state
    Update {
        /// Task ID
        task_id: String,

        /// New task state (pending, done, abandoned)
        #[arg(short, long)]
        state: String,

        /// Channel name (default: public)
        #[arg(short, long, default_value = "public")]
        channel: String,

        /// Server URL for remote operation
        #[arg(long, default_value = "")]
        server: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Manage task dependencies
    Dep {
        #[command(subcommand)]
        command: DepCommands,
    },

    /// List tasks ready to start (no blockers)
    Ready {
        /// Channel name (default: public)
        #[arg(short, long, default_value = "public")]
        channel: String,

        /// Server URL for remote operation
        #[arg(long, default_value = "")]
        server: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// List blocked tasks
    Blocked {
        /// Channel name (default: public)
        #[arg(short, long, default_value = "public")]
        channel: String,

        /// Server URL for remote operation
        #[arg(long, default_value = "")]
        server: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum DepCommands {
    /// Add a dependency between tasks
    Add {
        /// Child task ID
        child_id: String,

        /// Parent task ID
        parent_id: String,

        /// Dependency type (blocks, related, parent)
        #[arg(short, long, default_value = "blocks")]
        dependency_type: String,

        /// Channel name (default: public)
        #[arg(short, long, default_value = "public")]
        channel: String,

        /// Server URL for remote operation
        #[arg(long, default_value = "")]
        server: String,
    },

    /// Remove a dependency between tasks
    Remove {
        /// Child task ID
        child_id: String,

        /// Parent task ID
        parent_id: String,

        /// Channel name (default: public)
        #[arg(short, long, default_value = "public")]
        channel: String,

        /// Server URL for remote operation
        #[arg(long, default_value = "")]
        server: String,
    },

    /// Show dependency graph for a task
    Graph {
        /// Task ID
        task_id: String,

        /// Channel name (default: public)
        #[arg(short, long, default_value = "public")]
        channel: String,

        /// Server URL for remote operation
        #[arg(long, default_value = "")]
        server: String,
    },
}
