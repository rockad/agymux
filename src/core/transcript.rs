use anyhow::{Context, Result};
use std::io::BufRead;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptSummary {
    pub first_prompt: String,
    pub last_response: String,
    pub tools_used: Vec<String>,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
}

pub struct TranscriptParser;

impl TranscriptParser {
    /// Parse transcript summary given a conversation ID
    pub fn parse(conv_id: &str) -> Result<TranscriptSummary> {
        let path = Self::resolve_transcript_path(conv_id)?;
        Self::parse_file(&path)
    }

    /// Locate transcript.jsonl (or transcript_full.jsonl fallback) for a conversation ID
    pub fn resolve_transcript_path(conv_id: &str) -> Result<PathBuf> {
        let brain_dir = if let Ok(custom) = std::env::var("AGYMUX_BRAIN_DIR") {
            PathBuf::from(custom)
        } else {
            let home =
                dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Home directory not found"))?;
            home.join(".gemini/antigravity-cli/brain")
        };

        let primary = brain_dir
            .join(conv_id)
            .join(".system_generated/logs/transcript.jsonl");
        if primary.exists() {
            return Ok(primary);
        }

        let fallback = brain_dir
            .join(conv_id)
            .join(".system_generated/logs/transcript_full.jsonl");
        if fallback.exists() {
            return Ok(fallback);
        }

        anyhow::bail!(
            "Transcript file not found for conversation '{}' at {:?}",
            conv_id,
            primary
        )
    }

    /// Parse transcript summary directly from a file path
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<TranscriptSummary> {
        let file = std::fs::File::open(path.as_ref())
            .with_context(|| format!("Failed to open transcript file at {:?}", path.as_ref()))?;
        let reader = std::io::BufReader::new(file);
        Self::parse_reader(reader)
    }

    /// Stream line-by-line through a JSONL reader to assemble TranscriptSummary
    pub fn parse_reader<R: BufRead>(reader: R) -> Result<TranscriptSummary> {
        let mut first_prompt = String::new();
        let mut last_response = String::new();
        let mut tools_used = Vec::new();
        let mut total_input_tokens: u64 = 0;
        let mut total_output_tokens: u64 = 0;

        for line_res in reader.lines() {
            let line = line_res?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let val: serde_json::Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(_) => continue,
            };

            // Accumulate token usage
            if let Some(tokens) = val.get("input_tokens").and_then(|v| v.as_u64()) {
                total_input_tokens += tokens;
            }
            if let Some(tokens) = val.get("output_tokens").and_then(|v| v.as_u64()) {
                total_output_tokens += tokens;
            }

            // Accumulate executed tool names
            if let Some(tools) = val.get("tool_calls").and_then(|v| v.as_array()) {
                for tool in tools {
                    if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                        let name_str = name.to_string();
                        if !tools_used.contains(&name_str) {
                            tools_used.push(name_str);
                        }
                    }
                }
            }

            let step_type = val.get("type").and_then(|v| v.as_str()).unwrap_or_default();
            let step_source = val
                .get("source")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            // First prompt extraction
            if first_prompt.is_empty()
                && (step_type == "USER_INPUT" || step_source == "USER_EXPLICIT")
            {
                if let Some(content) = val.get("content").and_then(|v| v.as_str()) {
                    let cleaned = Self::clean_user_prompt(content);
                    if !cleaned.is_empty() {
                        first_prompt = cleaned;
                    }
                }
            }

            // Latest assistant response / turn extraction
            if step_type == "PLANNER_RESPONSE" {
                if let Some(content) = val.get("content").and_then(|v| v.as_str()) {
                    if !content.trim().is_empty() {
                        last_response = content.trim().to_string();
                    }
                } else if let Some(thinking) = val.get("thinking").and_then(|v| v.as_str()) {
                    if !thinking.trim().is_empty() {
                        last_response = thinking.trim().to_string();
                    }
                }
            }
        }

        Ok(TranscriptSummary {
            first_prompt,
            last_response,
            tools_used,
            total_input_tokens,
            total_output_tokens,
        })
    }

    /// Strip metadata XML envelope tags and extract genuine user request
    pub fn clean_user_prompt(raw: &str) -> String {
        let mut text = raw.to_string();

        // 1. If <USER_REQUEST> tags exist, extract inner content
        if let Some(start) = text.find("<USER_REQUEST>") {
            let after_start = start + "<USER_REQUEST>".len();
            if let Some(end) = text[after_start..].find("</USER_REQUEST>") {
                text = text[after_start..after_start + end].to_string();
            }
        }

        // 2. Strip any remaining <ADDITIONAL_METADATA> ... </ADDITIONAL_METADATA>
        while let Some(start) = text.find("<ADDITIONAL_METADATA>") {
            if let Some(end) = text.find("</ADDITIONAL_METADATA>") {
                let end_pos = end + "</ADDITIONAL_METADATA>".len();
                text.replace_range(start..end_pos, "");
            } else {
                break;
            }
        }

        // 3. Strip any <USER_SETTINGS_CHANGE> ... </USER_SETTINGS_CHANGE>
        while let Some(start) = text.find("<USER_SETTINGS_CHANGE>") {
            if let Some(end) = text.find("</USER_SETTINGS_CHANGE>") {
                let end_pos = end + "</USER_SETTINGS_CHANGE>".len();
                text.replace_range(start..end_pos, "");
            } else {
                break;
            }
        }

        text.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_clean_user_prompt() {
        let raw = "<USER_REQUEST>\nFix the broken build in driver.rs\n</USER_REQUEST>\n<ADDITIONAL_METADATA>\ntime: 2026-10-07\n</ADDITIONAL_METADATA>";
        let cleaned = TranscriptParser::clean_user_prompt(raw);
        assert_eq!(cleaned, "Fix the broken build in driver.rs");
    }

    #[test]
    fn test_clean_user_prompt_plain() {
        let raw = "  Run cargo test  ";
        let cleaned = TranscriptParser::clean_user_prompt(raw);
        assert_eq!(cleaned, "Run cargo test");
    }

    #[test]
    fn test_parse_reader() {
        let data = r#"
{"step_index":0,"source":"USER_EXPLICIT","type":"USER_INPUT","content":"<USER_REQUEST>\nImplement agymux\n</USER_REQUEST>"}
{"step_index":1,"source":"MODEL","type":"PLANNER_RESPONSE","input_tokens":100,"output_tokens":25,"content":"Starting implementation...","tool_calls":[{"name":"view_file"}]}
{"step_index":2,"source":"MODEL","type":"PLANNER_RESPONSE","input_tokens":200,"output_tokens":50,"content":"All done successfully!","tool_calls":[{"name":"write_to_file"},{"name":"view_file"}]}
"#;

        let cursor = Cursor::new(data);
        let summary = TranscriptParser::parse_reader(cursor).expect("parse should succeed");

        assert_eq!(summary.first_prompt, "Implement agymux");
        assert_eq!(summary.last_response, "All done successfully!");
        assert_eq!(summary.tools_used, vec!["view_file", "write_to_file"]);
        assert_eq!(summary.total_input_tokens, 300);
        assert_eq!(summary.total_output_tokens, 75);
    }

    #[test]
    fn test_parse_real_conversation_if_exists() {
        if let Ok(summary) = TranscriptParser::parse("39b4fed8-ed5e-4d60-bf2b-933492153879") {
            assert!(!summary.first_prompt.is_empty());
        }
    }

    #[test]
    fn test_parse_reader_malformed_and_empty_lines() {
        let data = r#"
malformed json string
{"invalid": json missing quote}

{"step_index":0,"source":"USER_EXPLICIT","type":"USER_INPUT","content":"Hello world"}
{"step_index":1,"source":"MODEL","type":"PLANNER_RESPONSE","thinking":"Thinking deeply...","tool_calls":[{"name":"ls"}]}
"#;
        let cursor = Cursor::new(data);
        let summary =
            TranscriptParser::parse_reader(cursor).expect("malformed lines should not abort");
        assert_eq!(summary.first_prompt, "Hello world");
        assert_eq!(summary.last_response, "Thinking deeply...");
        assert_eq!(summary.tools_used, vec!["ls"]);
        assert_eq!(summary.total_input_tokens, 0);
        assert_eq!(summary.total_output_tokens, 0);
    }

    #[test]
    fn test_parse_reader_duplicate_tools_and_empty() {
        let data = r#"
{"step_index":0,"source":"MODEL","type":"PLANNER_RESPONSE","tool_calls":[{"name":"tool_a"},{"name":"tool_a"},{"name":"tool_b"}]}
"#;
        let cursor = Cursor::new(data);
        let summary = TranscriptParser::parse_reader(cursor).expect("parse should succeed");
        assert_eq!(summary.tools_used, vec!["tool_a", "tool_b"]);

        let empty_cursor = Cursor::new("");
        let empty_summary =
            TranscriptParser::parse_reader(empty_cursor).expect("empty reader succeeds");
        assert!(empty_summary.first_prompt.is_empty());
        assert!(empty_summary.last_response.is_empty());
        assert!(empty_summary.tools_used.is_empty());
    }
}
