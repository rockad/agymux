use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "ax",
    author = "Aleksandr Korolev",
    version,
    disable_version_flag = true,
    about = "OpenCode-style tabbed cockpit for Google Antigravity & Claude Code (agEmux)",
    long_about = "A high-performance terminal multiplexer workspace manager for Google Antigravity (agy) and Claude Code (claude). Provides multi-turn tab navigation, instant window naming, and rich session search inside tmux."
)]
pub struct Cli {
    /// Print version information
    #[arg(short = 'v', short_alias = 'V', long = "version", action = clap::ArgAction::Version)]
    pub version: Option<bool>,

    /// Override project session name (default: git root or folder name)
    #[arg(short = 'p', long = "project", global = true)]
    pub project: Option<String>,

    /// Working directory (default: current working directory)
    #[arg(short = 'C', long = "dir", global = true)]
    pub dir: Option<PathBuf>,

    /// Direct execution without launching or attaching tmux
    #[arg(short = 'd', long = "direct", global = true)]
    pub direct: bool,

    /// Enable remote control mode (Google Antigravity web surface sync)
    #[arg(long = "remote-control", global = true)]
    pub remote_control: bool,

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

        /// Target agent harness (agy, claude, term)
        #[arg(short = 'H', long = "harness", default_value = "agy")]
        harness: String,

        /// Arguments forwarded to command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Internal process runner with instant prompt-based window naming
    Run {
        /// Target agent harness (agy, claude, term)
        #[arg(short = 'H', long = "harness", default_value = "agy")]
        harness: String,

        /// Arguments passed to harness command
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

    /// Bind a conversation ID to the active tmux tab
    Bind {
        /// Optional conversation ID (UUID). If omitted, opens interactive picker.
        conversation_id: Option<String>,
    },

    /// Safely close a window/tab by id or status range, unbinding from tab registry
    #[command(alias = "close-tab")]
    CloseWindow {
        /// Target window id or status range (e.g. "@1" or "close:@1")
        target: Option<String>,
    },

    /// Internal lifecycle hook invoked by Antigravity on turn stop
    #[command(hide = true)]
    Hook,

    /// Install or update agymux lifecycle hook in Antigravity configuration
    Install,
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
    fn test_cli_version_flag() {
        let err_v =
            Cli::try_parse_from(["agymux", "-v"]).expect_err("-v should trigger version display");
        assert_eq!(err_v.kind(), clap::error::ErrorKind::DisplayVersion);

        let err_upper =
            Cli::try_parse_from(["agymux", "-V"]).expect_err("-V should trigger version display");
        assert_eq!(err_upper.kind(), clap::error::ErrorKind::DisplayVersion);

        let err_long = Cli::try_parse_from(["agymux", "--version"])
            .expect_err("--version should trigger version display");
        assert_eq!(err_long.kind(), clap::error::ErrorKind::DisplayVersion);
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
        let cli = Cli::try_parse_from([
            "agymux",
            "new",
            "-n",
            "feature-tab",
            "-H",
            "claude",
            "--",
            "-c",
        ])
        .expect("parsing succeeds");
        match cli.command {
            Some(Commands::New {
                name,
                harness,
                args,
            }) => {
                assert_eq!(name.as_deref(), Some("feature-tab"));
                assert_eq!(harness, "claude");
                assert_eq!(args, vec!["-c"]);
            }
            other => panic!("expected New, got {:?}", other),
        }
    }

    #[test]
    fn test_cli_run_subcommand() {
        let cli = Cli::try_parse_from([
            "agymux",
            "run",
            "-H",
            "term",
            "--",
            "--conversation",
            "uuid-123",
            "hello",
        ])
        .expect("parsing succeeds");
        match cli.command {
            Some(Commands::Run { harness, args }) => {
                assert_eq!(harness, "term");
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
    fn test_cli_bind_subcommand() {
        let cli_no_id = Cli::try_parse_from(["agymux", "bind"]).expect("parsing succeeds");
        match cli_no_id.command {
            Some(Commands::Bind { conversation_id }) => {
                assert!(conversation_id.is_none());
            }
            other => panic!("expected Bind, got {:?}", other),
        }

        let cli_with_id =
            Cli::try_parse_from(["agymux", "bind", "conv-uuid-123"]).expect("parsing succeeds");
        match cli_with_id.command {
            Some(Commands::Bind { conversation_id }) => {
                assert_eq!(conversation_id.as_deref(), Some("conv-uuid-123"));
            }
            other => panic!("expected Bind, got {:?}", other),
        }
    }

    #[test]
    fn test_cli_remote_control_flag() {
        let cli_root =
            Cli::try_parse_from(["agymux", "--remote-control"]).expect("parsing succeeds");
        assert!(cli_root.remote_control);

        let cli_attach = Cli::try_parse_from(["agymux", "attach", "--remote-control"])
            .expect("parsing succeeds");
        assert!(cli_attach.remote_control);

        let cli_run =
            Cli::try_parse_from(["agymux", "run", "--remote-control"]).expect("parsing succeeds");
        assert!(cli_run.remote_control);
    }

    #[test]
    fn test_cli_hook_and_install_subcommands() {
        let cli_hook = Cli::try_parse_from(["agymux", "hook"]).expect("parsing succeeds");
        assert!(matches!(cli_hook.command, Some(Commands::Hook)));

        let cli_install = Cli::try_parse_from(["agymux", "install"]).expect("parsing succeeds");
        assert!(matches!(cli_install.command, Some(Commands::Install)));
    }

    #[test]
    fn test_cli_close_window_subcommand() {
        let cli =
            Cli::try_parse_from(["agymux", "close-window", "close:@1"]).expect("parsing succeeds");
        match cli.command {
            Some(Commands::CloseWindow { target }) => {
                assert_eq!(target.as_deref(), Some("close:@1"));
            }
            other => panic!("expected CloseWindow, got {:?}", other),
        }

        let cli_alias = Cli::try_parse_from(["agymux", "close-tab"]).expect("parsing succeeds");
        match cli_alias.command {
            Some(Commands::CloseWindow { target }) => {
                assert_eq!(target, None);
            }
            other => panic!("expected CloseWindow, got {:?}", other),
        }
    }
}
