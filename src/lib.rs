mod hook;
mod rollout;

#[cfg(test)]
mod test_support;

pub use hook::{
    create_hook_output, create_hook_output_with_debug, format_context_window, message_for_hook,
    HookInput, HookOutput, HookSpecificOutput, CONTEXT_COMPACTED_MESSAGE,
};
pub use rollout::{find_session_file, read_last_token_usage, TokenUsage};

use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

pub(crate) fn non_empty_env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

pub fn environment_codex_home() -> Option<PathBuf> {
    non_empty_env_path("CODEX_HOME")
}

pub fn environment_debug_enabled() -> bool {
    env::var_os("CODEX_CONTEXT_WINDOW_DEBUG")
        .map(|value: OsString| value == "1")
        .unwrap_or(false)
}
