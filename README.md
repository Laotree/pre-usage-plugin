# pre-usage

**pre-usage** is a Claude Code hook plugin that estimates token usage before each prompt
is sent, and warns (or blocks) when the estimate exceeds a configurable threshold.
Distributed under the MIT License.

## Core Functionality

The tool registers as a `UserPromptSubmit` hook. Before every prompt reaches Claude it:

1. **Reads session history** — scans the current session's JSONL log to sum all tokens
   consumed so far (`input + output + cache_creation + cache_read`).
2. **Estimates new prompt tokens** — approximates the incoming prompt size
   (`content bytes ÷ 4`).
3. **Compares against threshold** — checks the total against the configured threshold
   (default **50 000 tokens**).

## Strategy

Two strategies control what happens when the estimate exceeds the threshold:

### Warn (default)

Prints the estimate to stderr and **auto-proceeds** — no keypress required, prompt
is sent to Claude immediately.

```
⚠️ ~65K tokens — 30% over 50K threshold
```

### Block

Shows the estimate and asks for confirmation:

```
⚠️ ~65K tokens — 30% over 50K threshold
[S]end  [C]ancel  s[K]ip this session
```

| Choice | Action |
|---|---|
| **S** | Proceed — prompt is sent to Claude |
| **C** (or Enter) | Block — exits with code **2**, prompt is discarded |
| **K** | Skip this session — sends the prompt and suppresses further blocking for the rest of this session |

> **Skip** writes a marker file to the OS temp directory (`/tmp/pre-usage-skip-<session-id>`)
> so all subsequent prompts in the same session auto-proceed (warn mode). It is scoped to
> the session ID, so a new Claude Code session starts fresh. The marker is cleaned on reboot.

When there is no TTY (e.g. certain CI or IDE integrations), block mode falls back to a
macOS dialog via `osascript`.

## Exit codes

Claude Code's `UserPromptSubmit` hook protocol:

| Code | Meaning |
|------|---------|
| 0 | Proceed — prompt is sent |
| 2 | **Block** — prompt is discarded |

> **Note:** exit 1 is **not** a block in Claude Code — it is treated as a non-blocking
> error and the prompt proceeds. Only exit 2 discards the prompt.

## Installation

```bash
./install.sh
```

This builds the release binary, copies it to `~/.claude/plugins/pre-usage`, and
registers the `UserPromptSubmit` hook in `~/.claude/settings.json` automatically.

### Manual install

```bash
cargo build --release
cp target/release/pre-usage ~/.claude/plugins/pre-usage
```

Then add to `~/.claude/settings.json`:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          { "type": "command", "command": "/Users/<you>/.claude/plugins/pre-usage" }
        ]
      }
    ]
  }
}
```

## Configuration

### Environment variables

```bash
export PRE_USAGE_THRESHOLD=50K    # token threshold; default 50K
export PRE_USAGE_STRATEGY=warn    # warn (default) or block
```

`PRE_USAGE_THRESHOLD` accepts a plain integer **or** a human-readable suffix (`K` = ×1 000,
`M` = ×1 000 000). Suffixes are case-insensitive. Decimals are not supported.

Valid examples: `50000`, `50K`, `100K`, `1M`.

`PRE_USAGE_STRATEGY` values are case-insensitive:

| Value | Behaviour |
|-------|-----------|
| `warn` | Print estimate and auto-proceed (default) |
| `block` | Require explicit confirmation via Send/Cancel/Skip |

### Per-project config

Each project can override the strategy and/or threshold via `.claude/pre-usage.toml`:

```toml
# .claude/pre-usage.toml
threshold = "200K"
strategy = "block"
```

All fields are optional. Resolution order (first wins):

1. Project config (`.claude/pre-usage.toml`)
2. Environment variable
3. Hard default (`warn` / `50K`)

## Make Commands

| Command | Action |
|---|---|
| `make` | Debug build |
| `make release` | Release build |
| `make test` | Run tests |
| `make lint` | `cargo clippy` |
| `make fmt` | `cargo fmt` |
| `make clean` | `cargo clean` |
| `make install` | Run `install.sh` |
| `make hooks` | Install git pre-push hook |

## Uninstallation

```bash
rm ~/.claude/plugins/pre-usage
```

Then remove the `UserPromptSubmit` hook entry from `~/.claude/settings.json`.
