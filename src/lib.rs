mod hook;
mod rollout;

#[cfg(test)]
mod test_support;

pub use hook::{
    create_hook_output, create_hook_output_with_debug, format_context_window,
    format_context_window_limit, message_for_hook, HookInput, HookOutput, HookSpecificOutput,
    CONTEXT_COMPACTED_MESSAGE,
};
pub use rollout::{read_context_window_limit, read_last_token_usage, TokenUsage};

use std::env;
use std::ffi::OsString;

pub fn environment_debug_enabled() -> bool {
    env::var_os("CODEX_CONTEXT_WINDOW_DEBUG")
        .map(|value: OsString| value == "1")
        .unwrap_or(false)
}
