# Codex Context Window

English · [Русский](docs/ru/README.md)

Context windows fill up. When Codex compacts a long conversation to make room, important details, intermediate findings, constraints, and unfinished work can be reduced or lost. A task may then drift, repeat work, or even end with the wrong result.

By default, the model cannot see how much context remains. It does not know when compaction is approaching, so it has no chance to checkpoint its working state or adjust how it continues beforehand.

Codex Context Window closes that blind spot. Through lifecycle hooks, the model receives the current state of its own context window and can take that information into account while working. This awareness can help it make sure important intermediate results are not lost during compaction—this can reduce the risk of errors afterward.

The plugin does not save or restore information itself. It only provides a compact signal:

```text
<meta>YOUR CONTEXT WINDOW: 193800 / 258400 (75%)</meta>
```

Codex Context Window encourages the model to account for its context-window state so important information is not lost during compaction, **but it does not prescribe a specific response**. The model can decide whether to save intermediate state, where and how to store it, or whether preparing key information for compaction is enough.

By default, from 70% reported usage onward, the plugin asks the model to keep the risk of compaction in mind as it continues working, so information needed to continue correctly is not lost. If you prefer, you can define the desired behavior in `AGENTS.md` or the Codex system instructions: specify whether the model should save intermediate information to files and, if so, what to save, where, and how. Without such instructions, the model chooses the approach itself.

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

The placeholders below are replaced with values from the current session.

- `SessionStart` with source `startup` runs at the beginning of each new session and provides its effective context-window limit, while `SubagentStart` provides the limit for the new subagent from its own session file. Both inject:

  ```text
  <meta>YOUR CONTEXT WINDOW LIMIT IS {limit} TOKENS. AUTOMATIC COMPACTION MAY OCCUR BEFORE THE LIMIT IS REACHED. UNLESS APPLICABLE INSTRUCTIONS SPECIFY DIFFERENT BEHAVIOR, FROM 70% REPORTED USAGE ONWARD, KEEP THE RISK OF COMPACTION IN MIND AS YOU CONTINUE WORKING AND MAKE SURE CONTEXT COMPACTION DOES NOT CAUSE THE LOSS OF INFORMATION NEEDED TO CONTINUE WORK CORRECTLY, INCLUDING KEY REQUIREMENTS, DECISIONS, INTERMEDIATE RESULTS, AND TASK STATE.</meta>
  ```

- `UserPromptSubmit` and `PostToolUse` provide the latest available context-window usage:

  ```text
  <meta>YOUR CONTEXT WINDOW: {used} / {limit} ({percent}%)</meta>
  ```

- `SessionStart` with source `compact` runs after compaction. It reports the transition and restores the guidance that may have been removed from the previous context:

  ```text
  <meta>YOUR CONTEXT WAS JUST COMPACTED. MAKE SURE ALL INFORMATION NEEDED TO CONTINUE WORK CORRECTLY REMAINS AVAILABLE. YOUR CONTEXT WINDOW LIMIT IS {limit} TOKENS. AUTOMATIC COMPACTION MAY OCCUR BEFORE THE LIMIT IS REACHED. UNLESS APPLICABLE INSTRUCTIONS SPECIFY DIFFERENT BEHAVIOR, FROM 70% REPORTED USAGE ONWARD, KEEP THE RISK OF COMPACTION IN MIND AS YOU CONTINUE WORKING AND MAKE SURE CONTEXT COMPACTION DOES NOT CAUSE THE LOSS OF INFORMATION NEEDED TO CONTINUE WORK CORRECTLY, INCLUDING KEY REQUIREMENTS, DECISIONS, INTERMEDIATE RESULTS, AND TASK STATE.</meta>
  ```

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
