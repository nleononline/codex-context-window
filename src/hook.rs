use crate::rollout::{read_context_window_limit, read_last_token_usage, TokenUsage};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const META_OPEN: &str = "<meta>";
const META_CLOSE: &str = "</meta>";
const CONTEXT_COMPACTED_NOTICE: &str = "YOUR CONTEXT WAS JUST COMPACTED. MAKE SURE ALL INFORMATION NEEDED TO CONTINUE WORK CORRECTLY REMAINS AVAILABLE.";
const CONTEXT_WINDOW_POLICY: &str = "AUTOMATIC COMPACTION MAY OCCUR BEFORE THE LIMIT IS REACHED. UNLESS APPLICABLE INSTRUCTIONS SPECIFY DIFFERENT BEHAVIOR, FROM 70% REPORTED USAGE ONWARD, KEEP THE RISK OF COMPACTION IN MIND AS YOU CONTINUE WORKING AND MAKE SURE CONTEXT COMPACTION DOES NOT CAUSE THE LOSS OF INFORMATION NEEDED TO CONTINUE WORK CORRECTLY, INCLUDING KEY REQUIREMENTS, DECISIONS, INTERMEDIATE RESULTS, AND TASK STATE.";

#[derive(Debug, Default, Deserialize)]
pub struct HookInput {
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

fn format_context_window_guidance(limit: u64) -> String {
    format!("YOUR CONTEXT WINDOW LIMIT IS {limit} TOKENS. {CONTEXT_WINDOW_POLICY}")
}

pub fn format_context_window_limit(limit: u64) -> String {
    let guidance = format_context_window_guidance(limit);
    format!("{META_OPEN}{guidance}{META_CLOSE}")
}

pub fn format_compacted_context_window(limit: u64) -> String {
    let guidance = format_context_window_guidance(limit);
    format!("{META_OPEN}{CONTEXT_COMPACTED_NOTICE} {guidance}{META_CLOSE}")
}

fn format_compacted_notice() -> String {
    format!("{META_OPEN}{CONTEXT_COMPACTED_NOTICE}{META_CLOSE}")
}

fn append_debug_hook(message: String, hook_event_name: &str) -> String {
    let Some(content) = message.strip_suffix(META_CLOSE) else {
        return message;
    };

    format!("{content} (hook: {hook_event_name}){META_CLOSE}")
}

fn context_window_message(input: &HookInput) -> Option<String> {
    let session_file = input.transcript_path.as_deref()?;

    match read_last_token_usage(session_file) {
        Ok(Some(usage)) => Some(format_context_window(usage)),
        Ok(None) | Err(_) => None,
    }
}

fn context_window_limit(input: &HookInput) -> Option<u64> {
    let session_file = input.transcript_path.as_deref()?;

    match read_context_window_limit(session_file) {
        Ok(Some(limit)) => Some(limit),
        Ok(None) | Err(_) => None,
    }
}

fn context_window_limit_message(input: &HookInput) -> Option<String> {
    context_window_limit(input).map(format_context_window_limit)
}

fn compacted_context_window_message(input: &HookInput) -> String {
    context_window_limit(input)
        .map(format_compacted_context_window)
        .unwrap_or_else(format_compacted_notice)
}

pub fn message_for_hook(input: &HookInput) -> Option<String> {
    match (input.hook_event_name.as_deref(), input.source.as_deref()) {
        (Some("SessionStart"), Some("startup")) => context_window_limit_message(input),
        (Some("SubagentStart"), _) => context_window_limit_message(input),
        (Some("SessionStart"), Some("compact")) => Some(compacted_context_window_message(input)),
        (Some("UserPromptSubmit"), _) | (Some("PostToolUse"), _) => context_window_message(input),
        _ => None,
    }
}

pub fn create_hook_output(input: &HookInput) -> Option<HookOutput> {
    create_hook_output_with_debug(input, false)
}

pub fn create_hook_output_with_debug(input: &HookInput, debug_enabled: bool) -> Option<HookOutput> {
    let hook_event_name = input.hook_event_name.as_deref()?;
    let mut message = message_for_hook(input)?;
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
    use crate::test_support::{task_started, token_count, TempDirectory};
    use serde_json::json;
    use std::fs;

    const EXPECTED_CONTEXT_WINDOW_LIMIT: &str = "<meta>YOUR CONTEXT WINDOW LIMIT IS 258400 TOKENS. AUTOMATIC COMPACTION MAY OCCUR BEFORE THE LIMIT IS REACHED. UNLESS APPLICABLE INSTRUCTIONS SPECIFY DIFFERENT BEHAVIOR, FROM 70% REPORTED USAGE ONWARD, KEEP THE RISK OF COMPACTION IN MIND AS YOU CONTINUE WORKING AND MAKE SURE CONTEXT COMPACTION DOES NOT CAUSE THE LOSS OF INFORMATION NEEDED TO CONTINUE WORK CORRECTLY, INCLUDING KEY REQUIREMENTS, DECISIONS, INTERMEDIATE RESULTS, AND TASK STATE.</meta>";
    const EXPECTED_COMPACTED_CONTEXT_WINDOW: &str = "<meta>YOUR CONTEXT WAS JUST COMPACTED. MAKE SURE ALL INFORMATION NEEDED TO CONTINUE WORK CORRECTLY REMAINS AVAILABLE. YOUR CONTEXT WINDOW LIMIT IS 258400 TOKENS. AUTOMATIC COMPACTION MAY OCCUR BEFORE THE LIMIT IS REACHED. UNLESS APPLICABLE INSTRUCTIONS SPECIFY DIFFERENT BEHAVIOR, FROM 70% REPORTED USAGE ONWARD, KEEP THE RISK OF COMPACTION IN MIND AS YOU CONTINUE WORKING AND MAKE SURE CONTEXT COMPACTION DOES NOT CAUSE THE LOSS OF INFORMATION NEEDED TO CONTINUE WORK CORRECTLY, INCLUDING KEY REQUIREMENTS, DECISIONS, INTERMEDIATE RESULTS, AND TASK STATE.</meta>";
    const EXPECTED_COMPACTED_NOTICE: &str = "<meta>YOUR CONTEXT WAS JUST COMPACTED. MAKE SURE ALL INFORMATION NEEDED TO CONTINUE WORK CORRECTLY REMAINS AVAILABLE.</meta>";

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
    fn formats_startup_context_limit() {
        assert_eq!(
            format_context_window_limit(258_400),
            EXPECTED_CONTEXT_WINDOW_LIMIT
        );
    }

    #[test]
    fn formats_compacted_context_limit() {
        assert_eq!(
            format_compacted_context_window(258_400),
            EXPECTED_COMPACTED_CONTEXT_WINDOW
        );
    }

    #[test]
    fn returns_startup_context_limit() {
        let temp = TempDirectory::new();
        let rollout = temp.path().join("rollout-test-thread.jsonl");
        fs::write(&rollout, format!("{}\n", task_started(258_400))).unwrap();
        let startup = HookInput {
            transcript_path: Some(rollout),
            hook_event_name: Some("SessionStart".to_owned()),
            source: Some("startup".to_owned()),
        };

        assert_eq!(
            serde_json::to_value(create_hook_output(&startup).unwrap()).unwrap(),
            json!({
                "hookSpecificOutput": {
                    "hookEventName": "SessionStart",
                    "additionalContext": EXPECTED_CONTEXT_WINDOW_LIMIT
                }
            })
        );
    }

    #[test]
    fn returns_subagent_context_limit() {
        let temp = TempDirectory::new();
        let rollout = temp.path().join("rollout-test-subagent.jsonl");
        fs::write(&rollout, format!("{}\n", task_started(258_400))).unwrap();
        let subagent_start = HookInput {
            transcript_path: Some(rollout),
            hook_event_name: Some("SubagentStart".to_owned()),
            source: None,
        };

        assert_eq!(
            serde_json::to_value(create_hook_output(&subagent_start).unwrap()).unwrap(),
            json!({
                "hookSpecificOutput": {
                    "hookEventName": "SubagentStart",
                    "additionalContext": EXPECTED_CONTEXT_WINDOW_LIMIT
                }
            })
        );
    }

    #[test]
    fn returns_compaction_output() {
        let temp = TempDirectory::new();
        let rollout = temp.path().join("rollout-test-thread.jsonl");
        fs::write(&rollout, format!("{}\n", task_started(258_400))).unwrap();
        let compact = HookInput {
            transcript_path: Some(rollout),
            hook_event_name: Some("SessionStart".to_owned()),
            source: Some("compact".to_owned()),
        };
        assert_eq!(
            serde_json::to_value(create_hook_output(&compact).unwrap()).unwrap(),
            json!({
                "hookSpecificOutput": {
                    "hookEventName": "SessionStart",
                    "additionalContext": EXPECTED_COMPACTED_CONTEXT_WINDOW
                }
            })
        );
    }

    #[test]
    fn returns_compaction_notice_when_limit_is_unknown() {
        let compact = HookInput {
            hook_event_name: Some("SessionStart".to_owned()),
            source: Some("compact".to_owned()),
            ..HookInput::default()
        };

        assert_eq!(
            message_for_hook(&compact).as_deref(),
            Some(EXPECTED_COMPACTED_NOTICE)
        );
    }

    #[test]
    fn appends_originating_hook_in_debug_mode() {
        let temp = TempDirectory::new();
        let rollout = temp.path().join("rollout-test-thread.jsonl");
        fs::write(&rollout, format!("{}\n", task_started(258_400))).unwrap();
        let session_start = HookInput {
            transcript_path: Some(rollout),
            hook_event_name: Some("SessionStart".to_owned()),
            source: Some("compact".to_owned()),
        };
        let expected = EXPECTED_COMPACTED_CONTEXT_WINDOW.replacen(
            META_CLOSE,
            " (hook: SessionStart)</meta>",
            1,
        );
        assert_eq!(
            serde_json::to_value(create_hook_output_with_debug(&session_start, true).unwrap())
                .unwrap(),
            json!({
                "hookSpecificOutput": {
                    "hookEventName": "SessionStart",
                    "additionalContext": expected
                }
            })
        );
    }

    #[test]
    fn ignores_unconfigured_session_starts() {
        for source in ["clear", "resume"] {
            let input = HookInput {
                hook_event_name: Some("SessionStart".to_owned()),
                source: Some(source.to_owned()),
                ..HookInput::default()
            };

            assert_eq!(create_hook_output(&input), None);
        }
    }

    #[test]
    fn injects_usage_from_hook_transcript() {
        let temp = TempDirectory::new();
        let rollout = temp.path().join("transcript.jsonl");
        fs::write(&rollout, format!("{}\n", token_count(250, 1_000))).unwrap();

        let input = HookInput {
            transcript_path: Some(rollout),
            hook_event_name: Some("UserPromptSubmit".to_owned()),
            ..HookInput::default()
        };
        assert_eq!(
            serde_json::to_value(create_hook_output(&input).unwrap()).unwrap(),
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
            hook_event_name: Some("PostToolUse".to_owned()),
            ..HookInput::default()
        };

        assert_eq!(create_hook_output(&input), None);
    }

    #[test]
    fn emits_nothing_when_startup_limit_is_unknown() {
        let input = HookInput {
            hook_event_name: Some("SessionStart".to_owned()),
            source: Some("startup".to_owned()),
            ..HookInput::default()
        };

        assert_eq!(create_hook_output(&input), None);
    }

    #[test]
    fn ignores_unconfigured_hook_events() {
        for hook_event_name in ["PreToolUse", "SessionEnd"] {
            let input = HookInput {
                hook_event_name: Some(hook_event_name.to_owned()),
                ..HookInput::default()
            };

            assert_eq!(create_hook_output(&input), None);
        }
    }
}
