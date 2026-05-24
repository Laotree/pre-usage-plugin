# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build --release          # build the binary
cargo clippy                   # lint
cargo fmt                      # format
cargo test                     # run tests
./install.sh                   # build + install binary + register hook in settings.json
```

To test the binary directly (simulate a UserPromptSubmit hook call):
```bash
echo '{"prompt":"hello world","session_id":"test-123","cwd":"/tmp"}' \
  | ./target/release/pre-usage; echo "exit: $?"
```

## Architecture

`pre-usage` is a Rust binary installed at `~/.claude/plugins/pre-usage`.  
`install.sh` registers it as a `UserPromptSubmit` hook in `~/.claude/settings.json`:

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

Claude Code calls the binary **before every prompt is sent**. The binary reads hook JSON
from stdin, estimates token usage, and either exits 0 (proceed) or 1 (abort).

### Flow

```
UserPromptSubmit
      │
      ▼
  main.rs  ──reads stdin──▶  estimator.rs
                                  │
                          ┌───────┴────────┐
                          │                │
                   read session        estimate new
                   JSONL logs          prompt tokens
                   (accumulated        (content bytes ÷ 4)
                    session tokens)
                          │                │
                          └───────┬────────┘
                                  │
                            total_estimate
                                  │
                    ┌─────────────┴──────────────┐
                    │                            │
             ≤ threshold                   > threshold
                    │                            │
                exit 0                      ui.rs prompt
            (silent pass)              "[S]end  [C]ancel"
                                             │       │
                                           exit 0  exit 1
```

### Threshold

`PRE_USAGE_THRESHOLD` — environment variable, default **50 000 tokens**.  
Accepts a plain integer **or** a human-readable suffix (`K` = ×1 000, `M` = ×1 000 000).
Suffixes are case-insensitive. Decimals are not supported.

```bash
export PRE_USAGE_THRESHOLD=50K     # 50 000 (default)
export PRE_USAGE_THRESHOLD=100K    # 100 000
export PRE_USAGE_THRESHOLD=1M      # 1 000 000
```

An invalid value (e.g. `1.5M`, `abc`) prints a clear error and exits with code `2`.

### Strategy

`PRE_USAGE_STRATEGY` — what to do when the estimate exceeds the threshold.  
Default: **`warn`** (prints the estimate and auto-proceeds — no keypress required).

| Value | Behaviour |
|-------|-----------|
| `warn`  | Print the ⚠️ estimate to stderr and auto-proceed (exit 0, no keypress) (default) |
| `block` | Print the ⚠️ estimate and ask `[S]end / [C]ancel` (or macOS dialog if no TTY) |

```bash
export PRE_USAGE_STRATEGY=warn    # just a heads-up, no blocking (default)
export PRE_USAGE_STRATEGY=block   # require explicit confirmation
```

Values are case-insensitive. An invalid value prints a clear error and exits with code `2`.

### Modules

**`src/estimator.rs`** — all estimation logic; two steps run concurrently:
- **Session accumulation**: locates the current session JSONL under
  `~/.claude/projects/<encoded-cwd>/<session-id>.jsonl`, sums
  `input_tokens + output_tokens + cache_creation_input_tokens + cache_read_input_tokens`
  from every `assistant` message in the file.
- **Prompt estimation**: tokenises the incoming prompt text with a simple heuristic
  (`content.len() as u64 / 4`); accurate enough for a gate check.

**`src/ui.rs`** — all output goes to stderr. Interactive prompts read from `/dev/tty`
so they work even when stdin is the hook JSON pipe.

**`src/main.rs`** — parses the hook stdin JSON, calls `estimator::estimate()`, then
either returns immediately (below threshold), prints the estimate and auto-proceeds
(`warn` strategy), or calls `ui::confirm()` (`block` strategy).

### Hook stdin format (Claude Code)

```json
{
  "session_id": "<uuid>",
  "transcript_path": "/Users/<you>/.claude/projects/<encoded-cwd>/<session-id>.jsonl",
  "cwd": "/path/to/project",
  "hook_event_name": "UserPromptSubmit",
  "prompt": "<the user's prompt text>"
}
```

The binary reads `transcript_path` directly — no need to reconstruct the path from `cwd`.

### Exit codes

Claude Code's `UserPromptSubmit` hook protocol:

| Code | Claude Code meaning | When used |
|------|---------------------|-----------|
| `0`  | Proceed — send the prompt | Under threshold · warn mode · user chose Send |
| `2`  | **Block** — discard the prompt | User chose Cancel · bad config · no TTY in block mode |
| other | Non-blocking error (proceeds) | Internal errors (not used intentionally) |

> **Note:** exit `1` is **not** a block in Claude Code — it is treated as a non-blocking
> error and the prompt proceeds. Only exit `2` discards the prompt.

---

## Personalized AI Agents

Three specialized agents collaborate on this project. Invoke by name when needed.

### Amy — Project Manager

Amy ensures no code gets written based on a misunderstanding.

**Responsibilities:**
- Engage the user with clarifying questions until the request is fully understood
- Confirm scope, acceptance criteria, and edge cases before any code work begins
- Once understanding is confirmed, describe the task clearly

**When to invoke:** Any time a new feature request, bug report, or task arrives.

**Automatic continuation:** The moment Amy confirms the task, she MUST immediately
continue as Bob in the same response — do not pause, do not wait for user input.

---

### Bob — Engineer

Bob implements what's been scoped.

**Responsibilities:**
- Pick up tasks scoped by Amy
- Implement following existing code conventions and architecture
- Write or update tests alongside the code
- Keep commits focused and message them clearly
- Always work on a feature branch and open a PR

**When to invoke:** After Amy has scoped a task.

**Automatic continuation:** The moment Bob finishes implementation, he MUST immediately
continue as Con in the same response — do not pause, do not wait for user input.

**Hard rules:**
- NEVER push directly to main — all changes including docs and config
- Always work on a feature branch and open a PR
- PR must reference the issue/task it addresses

---

### Con — Reviewer

Con is the gatekeeper before anything merges.

**Responsibilities:**
- Review Bob's changes for correctness, style, and security
- Verify that all tests pass
- If criteria are met: approve; otherwise request changes
- Once approved and merged: clean up the feature branch

**Hard rules:**
- Con is the ONLY one who may merge to main
- Con must NEVER push directly to main
- Con must not merge until Amy (scope match) and Con (code quality) have approved

---

## Workflow

```
Amy clarifies → Amy confirms task → [continues as Bob] → Bob implements → [continues as Con] → Con reviews → Con merges + cleans up branch
```
