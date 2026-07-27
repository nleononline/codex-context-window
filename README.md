# Codex Context Window

English · [Русский](docs/ru/README.md)

Context windows fill up. When Codex compacts a long conversation to make room, important details, intermediate findings, constraints, and unfinished work can be reduced or lost. A task may then drift, repeat work, or even end with the wrong result.

By default, the model cannot see how much context remains. It does not know when compaction is approaching, so it has no chance to checkpoint its working state or adjust how it continues beforehand.

Codex Context Window closes that blind spot. Through lifecycle hooks, the model receives the current state of its own context window and can take that information into account while working. This awareness can help it preserve important intermediate results before compaction and reduce the risk of errors afterward.

The plugin does not save or restore information itself. It only provides a compact signal:

```text
<meta>YOUR CONTEXT WINDOW: 193800 / 258400 (75%)</meta>
```

## Installation

Codex Context Window is distributed through a Git-backed Codex plugin marketplace. Install the marketplace and the plugin from a terminal:

```bash
codex plugin marketplace add nleononline/codex-context-window --ref marketplace
codex plugin add codex-context-window@codex-context-window
```

> [!IMPORTANT]
> In Codex Desktop, open the installed plugin and select **Trust all** in its hooks list.

After granting access, open a new chat or restart Codex so the plugin hooks are loaded.

### Updating

Refresh the marketplace and reinstall the plugin:

```bash
codex plugin marketplace upgrade codex-context-window
codex plugin add codex-context-window@codex-context-window
```

## How it works

When Codex invokes a hook, its input normally includes `transcript_path`, the path to the current session file. The handler reads this JSONL file directly. If the field is missing or the path does not point to a file, it uses `session_id` from the same hook input and searches:

```text
${CODEX_HOME:-~/.codex}/sessions/**/rollout-*-${session_id}.jsonl
```

The fallback is accepted only when it finds exactly one matching file. The selected JSONL file is scanned backwards. From the newest record where `type == "event_msg"` and `payload.type == "token_count"`, the plugin reads:

```text
payload.info.last_token_usage.total_tokens
payload.info.model_context_window
```

If the session file or token data is missing or unreadable, the hook exits successfully without producing output.

The plugin reads local session data only and makes no network requests.

### Hook coverage

- `SessionStart` with source `compact`: `<meta>YOUR CONTEXT WAS JUST COMPACTED. VERIFY THAT THE TASK GOAL, REQUIREMENTS, DECISIONS, AND CURRENT PROGRESS WERE PRESERVED.</meta>`
- `UserPromptSubmit` and `PostToolUse`: current usage as model-visible `additionalContext`

Set `CODEX_CONTEXT_WINDOW_DEBUG=1` before starting Codex to append the originating hook to every signal and print handler errors to `stderr`:

```text
<meta>YOUR CONTEXT WINDOW: 193800 / 258400 (75%) (hook: UserPromptSubmit)</meta>
```

## Performance and overhead

The hook handler is written in Rust to minimize runtime overhead. Prebuilt, self-contained binaries are included for macOS, Linux, and Windows on Arm64 and x86-64. The platform launcher selects the correct binary automatically, so no additional runtime, tool, or package is required after installation.

The handler runs as a short-lived native process. It scans the transcript backwards in 64 KiB blocks and stops at the newest token-count event, so it does not normally read or parse the full session file.

In a 500-run reference benchmark on Apple Silicon macOS against a 7.8 MB active transcript, a complete usage hook invocation took 9.2 ms median and 12.3 ms p95, including the POSIX launcher and process startup. A lifecycle-only invocation took 9.0 ms median, indicating that process startup dominates the total overhead. Results vary by platform, storage, and system load.

## Compatibility

The plugin reads Codex session files. These files are currently stored as JSONL and use the `rollout-` filename prefix. Their internal structure may change between Codex versions. If the expected token data is no longer available, the plugin exits successfully without adding anything to the model context.

## Support

I spent considerable time studying how Codex works and investigating the issues that sometimes make it perform worse than it should. Codex Context Window is one practical result of that work. Rather than keeping it as a private tool, I released it as open source so everyone can use it.

If you find the project useful, you can support both the project and me as its author with a USDT donation:

- ERC20: `0x9687CF4d903c73D126847712dEd10078d43E9aFc`
- TON: `UQBEC4WAUr2smhYTxcUfgSQtPF0vz4B9lerO4sghXGyxyOTq`

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for local development and release instructions.
