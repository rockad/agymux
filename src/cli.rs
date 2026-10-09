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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_default_no_args() {
        let cli = Cli::try_parse_from(["agymux"]).expect("parsing succeeds");
        assert!(cli.command.is_none());
        assert!(cli.project.is_none());
        assert!(cli.dir.is_none());
        assert!(!cli.direct);
    }

    #[test]
    fn test_cli_global_flags() {
        let cli = Cli::try_parse_from([
            "agymux",
            "--project",
            "my-proj",
            "--dir",
            "/tmp/test",
            "--direct",
        ])
        .expect("parsing succeeds");
        assert_eq!(cli.project.as_deref(), Some("my-proj"));
        assert_eq!(cli.dir, Some(PathBuf::from("/tmp/test")));
        assert!(cli.direct);
    }

    #[test]
    fn test_cli_attach_subcommand_with_forwarded_args() {
        let cli = Cli::try_parse_from(["agymux", "attach", "--", "--model", "gemini-flash"])
            .expect("parsing succeeds");
        match cli.command {
            Some(Commands::Attach { args }) => {
                assert_eq!(args, vec!["--model", "gemini-flash"]);
            }
            other => panic!("expected Attach subcommand, got {:?}", other),
        }
    }

    #[test]
    fn test_cli_switch_subcommand() {
        let cli_default = Cli::try_parse_from(["agymux", "switch"]).expect("parsing succeeds");
        match cli_default.command {
            Some(Commands::Switch { scope }) => assert_eq!(scope, ScopeArg::Local),
            other => panic!("expected Switch local, got {:?}", other),
        }

        let cli_global =
            Cli::try_parse_from(["agymux", "switch", "-s", "global"]).expect("parsing succeeds");
        match cli_global.command {
            Some(Commands::Switch { scope }) => assert_eq!(scope, ScopeArg::Global),
            other => panic!("expected Switch global, got {:?}", other),
        }
    }

    #[test]
    fn test_cli_project_and_helper_subcommands() {
        let cli_proj = Cli::try_parse_from(["agymux", "project"]).expect("parsing succeeds");
        assert!(matches!(cli_proj.command, Some(Commands::Project)));

        let cli_help = Cli::try_parse_from(["agymux", "helper"]).expect("parsing succeeds");
        assert!(matches!(cli_help.command, Some(Commands::Helper)));

        let cli_ls = Cli::try_parse_from(["agymux", "ls"]).expect("parsing succeeds");
        assert!(matches!(cli_ls.command, Some(Commands::Ls)));
    }

    #[test]
    fn test_cli_palette_subcommand() {
        let cli = Cli::try_parse_from(["agymux", "palette"]).expect("parsing succeeds");
        match cli.command {
            Some(Commands::Palette { action }) => assert!(action.is_none()),
            other => panic!("expected Palette, got {:?}", other),
        }

        let cli_act =
            Cli::try_parse_from(["agymux", "palette", "-a", "rename"]).expect("parsing succeeds");
        match cli_act.command {
            Some(Commands::Palette { action }) => assert_eq!(action.as_deref(), Some("rename")),
            other => panic!("expected Palette rename, got {:?}", other),
        }
    }

    #[test]
    fn test_cli_new_subcommand() {
        let cli = Cli::try_parse_from(["agymux", "new", "-n", "feature-tab", "--", "-c"])
            .expect("parsing succeeds");
        match cli.command {
            Some(Commands::New { name, args }) => {
                assert_eq!(name.as_deref(), Some("feature-tab"));
                assert_eq!(args, vec!["-c"]);
            }
            other => panic!("expected New, got {:?}", other),
        }
    }

    #[test]
    fn test_cli_run_subcommand() {
        let cli =
            Cli::try_parse_from(["agymux", "run", "--", "--conversation", "uuid-123", "hello"])
                .expect("parsing succeeds");
        match cli.command {
            Some(Commands::Run { args }) => {
                assert_eq!(args, vec!["--conversation", "uuid-123", "hello"]);
            }
            other => panic!("expected Run, got {:?}", other),
        }
    }

    #[test]
    fn test_cli_preview_subcommand() {
        let cli =
            Cli::try_parse_from(["agymux", "preview", "conv-uuid-456"]).expect("parsing succeeds");
        match cli.command {
            Some(Commands::Preview { conversation_id }) => {
                assert_eq!(conversation_id, "conv-uuid-456");
            }
            other => panic!("expected Preview, got {:?}", other),
        }
    }

    #[test]
    fn test_cli_watch_subcommand() {
        let cli = Cli::try_parse_from([
            "agymux",
            "watch",
            "--window",
            "@1",
            "--project",
            "my-project",
            "--start-time",
            "2026-10-09 12:00:00",
            "--exclude-id",
            "last-uuid",
        ])
        .expect("parsing succeeds");
        match cli.command {
            Some(Commands::Watch {
                window,
                project,
                start_time,
                exclude_id,
            }) => {
                assert_eq!(window, "@1");
                assert_eq!(project, "my-project");
                assert_eq!(start_time, "2026-10-09 12:00:00");
                assert_eq!(exclude_id.as_deref(), Some("last-uuid"));
            }
            other => panic!("expected Watch, got {:?}", other),
        }
    }
}
