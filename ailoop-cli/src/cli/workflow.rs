//! Workflow CLI commands

use clap::Subcommand;

#[derive(Subcommand)]
pub enum WorkflowCommands {
    /// Start a workflow execution
    Start {
        /// Name of the workflow to execute
        workflow_name: String,

        /// Initiator identity
        #[arg(short, long, default_value = "cli-user")]
        initiator: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Get workflow execution status
    Status {
        /// Execution ID (UUID)
        execution_id: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// List available workflow definitions
    List {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Show workflow execution history
    History {
        /// Workflow name (optional, shows all if not specified)
        #[arg(short, long)]
        workflow: Option<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Approve a pending workflow approval request
    Approve {
        /// Approval request ID (UUID)
        approval_id: String,

        /// Operator identity
        #[arg(short, long, default_value = "cli-operator")]
        operator: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Deny a pending workflow approval request
    Deny {
        /// Approval request ID (UUID)
        approval_id: String,

        /// Operator identity
        #[arg(short, long, default_value = "cli-operator")]
        operator: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// List pending approval requests
    ListApprovals {
        /// Filter by execution ID (optional)
        #[arg(short, long)]
        execution: Option<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// View workflow execution logs (stdout/stderr)
    Logs {
        /// Execution ID (UUID)
        execution_id: String,

        /// State name (optional, shows all states if not specified)
        #[arg(short, long)]
        state: Option<String>,

        /// Number of recent lines to show (default: 100)
        #[arg(short, long, default_value = "100")]
        limit: usize,

        /// Skip first N lines
        #[arg(long, default_value = "0")]
        offset: usize,

        /// Follow output in real-time (like tail -f)
        #[arg(short, long)]
        follow: bool,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Display workflow execution metrics
    Metrics {
        /// Workflow name (optional, shows all workflows if not specified)
        #[arg(short, long)]
        workflow: Option<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Validate a workflow definition file
    Validate {
        /// Path to workflow YAML file
        workflow_file: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// List available workflow definition files
    ListDefs {
        /// Directory containing workflow definitions (default: ~/.ailoop/workflows)
        #[arg(short, long)]
        directory: Option<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
}
