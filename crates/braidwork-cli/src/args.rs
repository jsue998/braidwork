use crate::execution_args::{
    AgentCommand, AssignmentCommand, CapsuleCommand, DelegationCommand, DispatchArgs, IngestArgs,
    SessionCommand,
};
use braidwork_core::{
    id::{ResourceId, TaskId},
    resource::{AccessMode, ResourceStatus, Scarcity},
};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "braidwork",
    version,
    about = "Manage canonical AI resources and tasks in a local project",
    after_help = "Run `braidwork init [PATH]` first. Other commands discover the nearest project from the current directory, unless --project supplies an exact root."
)]
pub struct Cli {
    /// Open this exact project root instead of discovering parents (not for init)
    #[arg(long, global = true, value_name = "ROOT")]
    pub project: Option<PathBuf>,
    /// Emit JSON results on stdout; errors remain on stderr
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Register and inspect immutable agent definitions
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
    /// Register and inspect persistent execution conversations
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
    /// Allocate tasks and inspect historical allocations
    Assignment {
        #[command(subcommand)]
        command: AssignmentCommand,
    },
    /// Record and inspect explicit delegation acts
    Delegation {
        #[command(subcommand)]
        command: DelegationCommand,
    },
    /// Prepare and render portable task descriptions
    Capsule {
        #[command(subcommand)]
        command: CapsuleCommand,
    },
    /// Display a manual handoff; no browser automation
    Dispatch(DispatchArgs),
    /// Ingest an exact manually returned result, unverified by default
    Ingest(IngestArgs),
    /// Initialize an existing directory without overwriting state
    Init {
        /// Directory to initialize (defaults to the current directory)
        path: Option<PathBuf>,
        /// Human project name (defaults to the root directory name)
        #[arg(long)]
        name: Option<String>,
    },
    /// Show project versions and resource/task counts
    Status,
    /// Register and inspect AI resources; does not contact providers
    Resource {
        #[command(subcommand)]
        command: ResourceCommand,
    },
    /// Register and inspect project tasks; does not execute work
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
}

#[derive(Subcommand)]
pub enum ResourceCommand {
    /// Insert a resource with declared capacity and availability
    Add(ResourceArgs),
    /// List all resources in identifier order
    List,
    /// Show one resource by its exact identifier
    Show {
        /// Resource identifier
        id: ResourceId,
    },
}
#[derive(Args)]
pub struct ResourceArgs {
    /// Consumer-supplied resource identifier
    pub id: ResourceId,
    /// Human-readable name
    #[arg(long)]
    pub name: String,
    /// Provider label, without provider-specific configuration
    #[arg(long)]
    pub provider: String,
    /// Declared means of access
    #[arg(long, value_enum, default_value = "manual")]
    pub access_mode: Access,
    /// Declared scarcity
    #[arg(long, value_enum, default_value = "normal")]
    pub scarcity: ScarcityArg,
    /// Declared availability
    #[arg(long, value_enum, default_value = "available")]
    pub status: Availability,
}
#[derive(Subcommand)]
pub enum TaskCommand {
    /// Insert a pending task; referenced parent and dependencies must exist
    Add(TaskArgs),
    /// List all tasks in identifier order
    List,
    /// Show one task and its relationships by exact identifier
    Show {
        /// Task identifier
        id: TaskId,
    },
}
#[derive(Args)]
pub struct TaskArgs {
    /// Consumer-supplied task identifier
    pub id: TaskId,
    /// Human-readable title
    #[arg(long)]
    pub title: String,
    /// Intended result of the work
    #[arg(long)]
    pub objective: String,
    /// Existing enclosing task (does not imply dependency)
    #[arg(long)]
    pub parent: Option<TaskId>,
    /// Existing prerequisite task; repeat this flag for multiple prerequisites
    #[arg(long, value_name = "TASK_ID")]
    pub depends_on: Vec<TaskId>,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Access {
    Manual,
    Api,
    Harness,
    Local,
}
impl From<Access> for AccessMode {
    fn from(value: Access) -> Self {
        match value {
            Access::Manual => Self::Manual,
            Access::Api => Self::Api,
            Access::Harness => Self::Harness,
            Access::Local => Self::Local,
        }
    }
}
#[derive(Clone, Copy, ValueEnum)]
pub enum ScarcityArg {
    Abundant,
    Normal,
    Scarce,
    Critical,
}
impl From<ScarcityArg> for Scarcity {
    fn from(value: ScarcityArg) -> Self {
        match value {
            ScarcityArg::Abundant => Self::Abundant,
            ScarcityArg::Normal => Self::Normal,
            ScarcityArg::Scarce => Self::Scarce,
            ScarcityArg::Critical => Self::Critical,
        }
    }
}
#[derive(Clone, Copy, ValueEnum)]
pub enum Availability {
    Available,
    Unavailable,
    Exhausted,
}
impl From<Availability> for ResourceStatus {
    fn from(value: Availability) -> Self {
        match value {
            Availability::Available => Self::Available,
            Availability::Unavailable => Self::Unavailable,
            Availability::Exhausted => Self::Exhausted,
        }
    }
}
