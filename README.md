# pre-usage

**pre-usage** is a Claude Code hook plugin that estimates token usage before each prompt
is sent, and prompts for confirmation when the estimate exceeds a configurable threshold.
Distributed under the MIT License.

## Core Functionality

The tool registers as a `UserPromptSubmit` hook. Before every prompt reaches Claude it:

1. **Reads session history** — scans the current session's JSONL log to sum all tokens
   consumed so far (`input + output + cache_creation + cache_read`).
2. **Estimates new prompt tokens** — approximates the incoming prompt size
   (`content bytes ÷ 4`).
3. **Compares against threshold** — checks the total against `PRE_USAGE_THRESHOLD`
   (default **100 000 tokens**).

## Behaviour

| Estimate | Result |
|---|---|
| ≤ threshold | Silent pass — Claude receives the prompt immediately |
| > threshold | Shows estimate and asks **[S]end  [C]ancel** |

Choosing **C** (or Enter) exits with code 1, aborting the prompt.

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

```bash
export PRE_USAGE_THRESHOLD=50000   # tokens; default 100000
```

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
