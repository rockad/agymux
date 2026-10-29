use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AgentHarness {
    #[default]
    Antigravity,
    ClaudeCode,
    PlainTerminal,
}

impl AgentHarness {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Antigravity => "agy",
            Self::ClaudeCode => "claude",
            Self::PlainTerminal => "term",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        let trimmed = s.trim().to_lowercase();
        match trimmed.as_str() {
            "claude" | "claudecode" | "claude-code" => Self::ClaudeCode,
            "term" | "terminal" | "shell" | "sh" | "bash" | "zsh" => Self::PlainTerminal,
            _ => Self::Antigravity,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Antigravity => "󰚩",
            Self::ClaudeCode => "󰊠",
            Self::PlainTerminal => "",
        }
    }

    pub fn default_command(&self) -> &'static str {
        match self {
            Self::Antigravity => "agy",
            Self::ClaudeCode => "claude",
            Self::PlainTerminal => "$SHELL",
        }
    }
}

impl fmt::Display for AgentHarness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_harness_strings_and_loose_parsing() {
        assert_eq!(
            AgentHarness::from_str_loose("agy"),
            AgentHarness::Antigravity
        );
        assert_eq!(
            AgentHarness::from_str_loose("antigravity"),
            AgentHarness::Antigravity
        );
        assert_eq!(
            AgentHarness::from_str_loose("claude"),
            AgentHarness::ClaudeCode
        );
        assert_eq!(
            AgentHarness::from_str_loose("claudecode"),
            AgentHarness::ClaudeCode
        );
        assert_eq!(
            AgentHarness::from_str_loose("term"),
            AgentHarness::PlainTerminal
        );
        assert_eq!(
            AgentHarness::from_str_loose("terminal"),
            AgentHarness::PlainTerminal
        );
        assert_eq!(
            AgentHarness::from_str_loose("shell"),
            AgentHarness::PlainTerminal
        );
        assert_eq!(
            AgentHarness::from_str_loose("unknown"),
            AgentHarness::Antigravity
        );

        assert_eq!(AgentHarness::Antigravity.as_str(), "agy");
        assert_eq!(AgentHarness::ClaudeCode.as_str(), "claude");
        assert_eq!(AgentHarness::PlainTerminal.as_str(), "term");
    }

    #[test]
    fn test_agent_harness_icons_and_commands() {
        assert_eq!(AgentHarness::Antigravity.icon(), "󰚩");
        assert_eq!(AgentHarness::ClaudeCode.icon(), "󰊠");
        assert_eq!(AgentHarness::PlainTerminal.icon(), "");

        assert_eq!(AgentHarness::Antigravity.default_command(), "agy");
        assert_eq!(AgentHarness::ClaudeCode.default_command(), "claude");
        assert_eq!(AgentHarness::PlainTerminal.default_command(), "$SHELL");
    }
}
