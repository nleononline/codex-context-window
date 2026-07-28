# Codex Context Window

English · [Русский](docs/ru/README.md)

Context windows fill up. When Codex compacts a long conversation to make room, important details, intermediate findings, constraints, and unfinished work can be reduced or lost. A task may then drift, repeat work, or even end with the wrong result.

By default, the model cannot see how much context remains. It does not know when compaction is approaching, so it has no chance to checkpoint its working state or adjust how it continues beforehand.

Codex Context Window closes that blind spot. Through lifecycle hooks, the model receives the current state of its own context window and can take that information into account while working. This awareness can help it preserve important intermediate results before compaction and reduce the risk of errors afterward.

The plugin does not save or restore information itself. It only provides a compact signal:

```text
<meta>YOUR CONTEXT WINDOW: 193800 / 258400 (75%)</meta>
```

Codex Context Window provides awareness without prescribing a fixed workflow. It does not choose a usage threshold, storage location, or checkpoint format. If you want specific behavior, define it in `AGENTS.md` or your Codex system instructions, for example:

```text
When the context window reaches 70% usage, save intermediate results, task state, key requirements, and anything else needed to continue working correctly without losing progress.
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

The plugin uses lifecycle hooks to give the model context-window information while it works.

The handler is written in Rust and runs only for the duration of each hook invocation. It reads the session file backwards in 64 KiB blocks and stops at the first relevant event, so it does not read the entire session file.

The plugin includes prebuilt binaries for macOS, Linux, and Windows on Arm64 and x86-64, so no additional dependencies are required.

**All data is read locally from the current session, and the plugin makes no network requests.**

### Hook coverage

- `SessionStart` with source `startup`: provides the effective context-window limit and warns that automatic compaction may happen before reported usage reaches it.
- `SubagentStart`: provides the effective context-window limit for the new subagent from its own session file.
- `UserPromptSubmit` and `PostToolUse`: provide the latest available context-window usage.
- `SessionStart` with source `compact`: reminds the model to verify that the task goal, requirements, decisions, and current progress were not lost.

### Debug mode

Set `CODEX_CONTEXT_WINDOW_DEBUG=1` before starting Codex to see which hook produced each signal:

```text
<meta>YOUR CONTEXT WINDOW: 193800 / 258400 (75%) (hook: UserPromptSubmit)</meta>
```

## Performance and overhead

In a 500-run Apple Silicon benchmark with a 7.8 MB session file, the median time for a complete hook invocation was 9.2 ms. Most of that time was process startup. Actual results depend on your system.

## Compatibility

The plugin reads Codex session files, whose format may change after an app update. If the plugin can no longer obtain the needed data, it simply adds nothing to the model context and does not interrupt Codex.

## Support

I spent considerable time studying how Codex works and investigating the issues that sometimes make it perform worse than it should. Codex Context Window is one practical result of that work. Rather than keeping it as a private tool, I released it as open source so everyone can use it.

If you find the project useful, you can support both the project and me as its author with a USDT donation:

- ERC20: `0x9687CF4d903c73D126847712dEd10078d43E9aFc`
- TON: `UQBEC4WAUr2smhYTxcUfgSQtPF0vz4B9lerO4sghXGyxyOTq`

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for local development and release instructions.
