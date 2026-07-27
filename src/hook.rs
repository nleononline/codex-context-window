use crate::rollout::{find_session_file, read_last_token_usage, TokenUsage};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const META_OPEN: &str = "<meta>";
const META_CLOSE: &str = "</meta>";

pub const CONTEXT_COMPACTED_MESSAGE: &str = "<meta>YOUR CONTEXT WAS JUST COMPACTED. VERIFY THAT THE TASK GOAL, REQUIREMENTS, DECISIONS, AND CURRENT PROGRESS WERE PRESERVED.</meta>";

#[derive(Debug, Default, Deserialize)]
pub struct HookInput {
    pub session_id: Option<String>,
    pub transcript_path: Option<PathBuf>,
    pub hook_event_name: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct HookSpecificOutput {
    #[serde(rename = "hookEventName")]
    pub hook_event_name: String,
    #[serde(rename = "additionalContext")]
    pub additional_context: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct HookOutput {
    #[serde(rename = "hookSpecificOutput")]
    pub hook_specific_output: HookSpecificOutput,
}

pub fn format_context_window(usage: TokenUsage) -> String {
    let mut percent = format!("{:.1}", (usage.used as f64 / usage.limit as f64) * 100.0);
    if percent.ends_with(".0") {
        percent.truncate(percent.len() - 2);
    }

    format!(
        "{META_OPEN}YOUR CONTEXT WINDOW: {} / {} ({}%){META_CLOSE}",
        usage.used, usage.limit, percent
    )
}

fn append_debug_hook(message: String, hook_event_name: &str) -> String {
    let Some(content) = message.strip_suffix(META_CLOSE) else {
        return message;
    };

    format!("{content} (hook: {hook_event_name}){META_CLOSE}")
}

fn context_window_message(input: &HookInput, codex_home: Option<&Path>) -> Option<String> {
    let session_file = find_session_file(
        input.transcript_path.as_deref(),
        input.session_id.as_deref(),
        codex_home,
    )?;

    match read_last_token_usage(&session_file) {
        Ok(Some(usage)) => Some(format_context_window(usage)),
        Ok(None) | Err(_) => None,
    }
}

pub fn message_for_hook(input: &HookInput, codex_home: Option<&Path>) -> Option<String> {
    match (input.hook_event_name.as_deref(), input.source.as_deref()) {
        (Some("SessionStart"), Some("compact")) => Some(CONTEXT_COMPACTED_MESSAGE.to_owned()),
        (Some("UserPromptSubmit"), _) | (Some("PostToolUse"), _) => {
            context_window_message(input, codex_home)
        }
        _ => None,
    }
}

pub fn create_hook_output(input: &HookInput, codex_home: Option<&Path>) -> Option<HookOutput> {
    create_hook_output_with_debug(input, codex_home, false)
}

pub fn create_hook_output_with_debug(
    input: &HookInput,
    codex_home: Option<&Path>,
    debug_enabled: bool,
) -> Option<HookOutput> {
    let hook_event_name = input.hook_event_name.as_deref()?;
    let mut message = message_for_hook(input, codex_home)?;
    if debug_enabled {
        message = append_debug_hook(message, hook_event_name);
    }

    Some(HookOutput {
        hook_specific_output: HookSpecificOutput {
            hook_event_name: hook_event_name.to_owned(),
            additional_context: message,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{token_count, TempDirectory};
    use serde_json::json;
    use std::fs;

    #[test]
    fn formats_requested_context_marker() {
        assert_eq!(
            format_context_window(TokenUsage {
                used: 118_475,
                limit: 258_400,
            }),
            "<meta>YOUR CONTEXT WINDOW: 118475 / 258400 (45.8%)</meta>"
        );
        assert_eq!(
            format_context_window(TokenUsage {
                used: 500,
                limit: 1_000,
            }),
            "<meta>YOUR CONTEXT WINDOW: 500 / 1000 (50%)</meta>"
        );
    }

    #[test]
    fn returns_compaction_output() {
        let compact = HookInput {
            hook_event_name: Some("SessionStart".to_owned()),
            source: Some("compact".to_owned()),
            ..HookInput::default()
        };
        assert_eq!(
            serde_json::to_value(create_hook_output(&compact, None).unwrap()).unwrap(),
            json!({
                "hookSpecificOutput": {
                    "hookEventName": "SessionStart",
                    "additionalContext":
                        "<meta>YOUR CONTEXT WAS JUST COMPACTED. VERIFY THAT THE TASK GOAL, REQUIREMENTS, DECISIONS, AND CURRENT PROGRESS WERE PRESERVED.</meta>"
                }
            })
        );
    }

    #[test]
    fn appends_originating_hook_in_debug_mode() {
        let session_start = HookInput {
            hook_event_name: Some("SessionStart".to_owned()),
            source: Some("compact".to_owned()),
            ..HookInput::default()
        };
        assert_eq!(
            serde_json::to_value(
                create_hook_output_with_debug(&session_start, None, true).unwrap()
            )
            .unwrap(),
            json!({
                "hookSpecificOutput": {
                    "hookEventName": "SessionStart",
                    "additionalContext":
                        "<meta>YOUR CONTEXT WAS JUST COMPACTED. VERIFY THAT THE TASK GOAL, REQUIREMENTS, DECISIONS, AND CURRENT PROGRESS WERE PRESERVED. (hook: SessionStart)</meta>"
                }
            })
        );
    }

    #[test]
    fn ignores_non_compaction_session_starts() {
        for source in ["startup", "clear", "resume"] {
            let input = HookInput {
                hook_event_name: Some("SessionStart".to_owned()),
                source: Some(source.to_owned()),
                ..HookInput::default()
            };

            assert_eq!(create_hook_output(&input, None), None);
        }
    }

    #[test]
    fn injects_usage_for_model_visible_hook() {
        let temp = TempDirectory::new();
        let session_id = "session-123";
        let session_directory = temp.path().join("sessions/2026/07/27");
        fs::create_dir_all(&session_directory).unwrap();
        let rollout = session_directory.join(format!("rollout-current-{session_id}.jsonl"));
        fs::write(&rollout, format!("{}\n", token_count(250, 1_000))).unwrap();

        let input = HookInput {
            session_id: Some(session_id.to_owned()),
            hook_event_name: Some("UserPromptSubmit".to_owned()),
            ..HookInput::default()
        };
        assert_eq!(
            serde_json::to_value(create_hook_output(&input, Some(temp.path())).unwrap()).unwrap(),
            json!({
                "hookSpecificOutput": {
                    "hookEventName": "UserPromptSubmit",
                    "additionalContext":
                        "<meta>YOUR CONTEXT WINDOW: 250 / 1000 (25%)</meta>"
                }
            })
        );
    }

    #[test]
    fn emits_nothing_when_usage_is_unknown() {
        let input = HookInput {
            session_id: Some("missing-session".to_owned()),
            hook_event_name: Some("PostToolUse".to_owned()),
            ..HookInput::default()
        };

        assert_eq!(
            create_hook_output(&input, Some(Path::new("/definitely/missing"))),
            None
        );
    }

    #[test]
    fn ignores_unconfigured_hook_events() {
        for hook_event_name in ["PreToolUse", "SubagentStart"] {
            let input = HookInput {
                hook_event_name: Some(hook_event_name.to_owned()),
                ..HookInput::default()
            };

            assert_eq!(create_hook_output(&input, None), None);
        }
    }
}
