use codex_context_window::{
    create_hook_output_with_debug, environment_codex_home, environment_debug_enabled, HookInput,
};
use std::io::{self, Read, Write};

fn debug(message: &str) {
    if environment_debug_enabled() {
        eprintln!("{message}");
    }
}

fn main() {
    let mut stdin = Vec::new();
    if let Err(error) = io::stdin().read_to_end(&mut stdin) {
        debug(&format!("failed to read hook input: {error}"));
        return;
    }

    let input = match serde_json::from_slice::<HookInput>(&stdin) {
        Ok(input) => input,
        Err(error) => {
            debug(&format!("failed to parse hook input: {error}"));
            return;
        }
    };

    let codex_home = environment_codex_home();
    let Some(output) =
        create_hook_output_with_debug(&input, codex_home.as_deref(), environment_debug_enabled())
    else {
        return;
    };

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    if let Err(error) = serde_json::to_writer(&mut stdout, &output) {
        debug(&format!("failed to serialize hook output: {error}"));
        return;
    }
    let _ = writeln!(stdout);
}
