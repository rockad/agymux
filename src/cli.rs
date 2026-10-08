use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "agymux",
    author = "Aleksandr Korolev",
    version,
    about = "OpenCode-style tabbed cockpit for Google Antigravity",
    long_about = "A high-performance terminal multiplexer workspace manager for Google Antigravity (agy). Provides multi-turn tab navigation, instant window naming, and rich session search inside tmux."
)]
pub struct Cli {
    /// Override project session name (default: git root or folder name)
    #[arg(short = 'p', long = "project", global = true)]
    pub project: Option<String>,

    /// Working directory (default: current working directory)
    #[arg(short = 'C', long = "dir", global = true)]
    pub dir: Option<PathBuf>,

    /// Direct execution without launching or attaching tmux
    #[arg(short = 'd', long = "direct", global = true)]
    pub direct: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Attach to active session or start session in current directory (default)
    Attach {
        /// Optional arguments forwarded to agy
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Open two-pane conversation & tab picker modal
    Switch {
        /// Scope filter ("local" for current project, "global" for all projects)
        #[arg(short = 's', long = "scope", default_value = "local")]
        scope: ScopeArg,
    },

    /// Open project switcher modal
    Project,

    /// Open quick command palette modal
    Palette {
        /// Specific action name to invoke directly
        #[arg(short = 'a', long = "action")]
        action: Option<String>,
    },

    /// Display keyboard shortcuts cheatsheet modal
    Helper,

    /// Open a new conversation tab in current session
    New {
        /// Initial tab name
        #[arg(short = 'n', long = "name")]
        name: Option<String>,

        /// Arguments forwarded to agy
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Internal process runner with instant prompt-based window naming
    Run {
        /// Arguments passed to agy
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Render rich preview for a conversation ID
    Preview {
        /// Conversation ID UUID
        conversation_id: String,
    },

    /// List active agymux sessions
    Ls,

    /// Internal background watcher for renaming windows
    #[command(hide = true)]
    Watch {
        #[arg(long)]
        window: String,
        #[arg(long)]
        project: String,
        #[arg(long)]
        start_time: String,
        #[arg(long)]
        exclude_id: Option<String>,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeArg {
    Local,
    Global,
}
