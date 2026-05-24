use serde::Deserialize;
use std::path::Path;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};

const DEFAULT_THRESHOLD: u64 = 50_000;

/// What to do when the token estimate exceeds the threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// Show estimate + interactive `[S]end / [C]ancel` prompt (or osascript dialog).
    Block,
    /// Print the estimate to stderr and auto-proceed without user input (default).
    Warn,
}

/// Parse a `PRE_USAGE_STRATEGY` value (case-insensitive).
///
/// Accepted values: `"block"`, `"warn"`.
pub fn parse_strategy(s: &str) -> Result<Strategy, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "block" => Ok(Strategy::Block),
        "warn" => Ok(Strategy::Warn),
        other => Err(format!(
            "unknown strategy \"{other}\" — use \"block\" or \"warn\""
        )),
    }
}

/// Read `PRE_USAGE_STRATEGY` from the environment (default: `warn`).
///
/// Exits with code 2 and a clear message if the value is present but invalid.
pub fn strategy() -> Strategy {
    match std::env::var("PRE_USAGE_STRATEGY") {
        Err(_) => Strategy::Warn,
        Ok(raw) => match parse_strategy(&raw) {
            Ok(s) => s,
            Err(reason) => {
                eprintln!("pre-usage: invalid PRE_USAGE_STRATEGY \"{raw}\" — {reason}.");
                std::process::exit(2);
            }
        },
    }
}

/// Token fields from an assistant message's `usage` object in the JSONL log.
#[derive(Debug, Default, Deserialize)]
struct Usage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
}

/// Minimal shape of a JSONL log entry — we only care about assistant messages.
#[derive(Deserialize)]
struct LogEntry {
    #[serde(default)]
    r#type: String,
    #[serde(default)]
    message: Option<MessageBody>,
}

#[derive(Deserialize)]
struct MessageBody {
    #[serde(default)]
    role: String,
    usage: Option<Usage>,
}

pub struct Estimate {
    /// Tokens accumulated by previous turns in this session.
    pub session_tokens: u64,
    /// Rough estimate of the new prompt's token cost.
    pub prompt_tokens: u64,
    /// Configured gate threshold.
    pub threshold: u64,
}

impl Estimate {
    pub fn total(&self) -> u64 {
        self.session_tokens + self.prompt_tokens
    }

    pub fn exceeds_threshold(&self) -> bool {
        self.total() > self.threshold
    }
}

/// Parse a threshold string that may carry a `K`/`k` (×1 000) or `M`/`m` (×1 000 000) suffix.
///
/// # Examples
/// ```
/// assert_eq!(parse_threshold("100000"), Ok(100_000));
/// assert_eq!(parse_threshold("200K"),   Ok(200_000));
/// assert_eq!(parse_threshold("200k"),   Ok(200_000));
/// assert_eq!(parse_threshold("1M"),     Ok(1_000_000));
/// assert_eq!(parse_threshold("1m"),     Ok(1_000_000));
/// assert!(parse_threshold("1.5M").is_err());
/// assert!(parse_threshold("abc").is_err());
/// ```
pub fn parse_threshold(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty string".to_string());
    }

    let (digits, multiplier) = match s.chars().last() {
        Some('K') | Some('k') => (&s[..s.len() - 1], 1_000_u64),
        Some('M') | Some('m') => (&s[..s.len() - 1], 1_000_000_u64),
        _ => (s, 1_u64),
    };

    let base: u64 = digits
        .parse()
        .map_err(|_| format!("not a valid integer: \"{digits}\""))?;

    base.checked_mul(multiplier)
        .ok_or_else(|| format!("\"{s}\" overflows u64"))
}

/// Read `PRE_USAGE_THRESHOLD` from the environment (default 100 000).
///
/// Accepts plain integers (`100000`) or human-readable suffixes (`200K`, `1M`).
/// Exits with code 2 and a clear message if the value is present but invalid.
pub fn threshold() -> u64 {
    match std::env::var("PRE_USAGE_THRESHOLD") {
        Err(_) => DEFAULT_THRESHOLD, // not set → use default
        Ok(raw) => match parse_threshold(&raw) {
            Ok(v) => v,
            Err(reason) => {
                eprintln!(
                    "pre-usage: invalid PRE_USAGE_THRESHOLD \"{raw}\" — {reason}\n  \
                     Use a plain number (100000) or a K/M suffix (200K, 1M)."
                );
                std::process::exit(2);
            }
        },
    }
}

/// Estimate token usage for the upcoming prompt submission.
///
/// * `transcript_path` — path to the current session's JSONL file
/// * `prompt`          — the raw prompt text the user is about to send
pub async fn estimate(transcript_path: &str, prompt: &str) -> Estimate {
    let session_tokens = sum_session_tokens(transcript_path).await;
    let prompt_tokens = prompt.len() as u64 / 4;

    Estimate {
        session_tokens,
        prompt_tokens,
        threshold: threshold(),
    }
}

/// Sum all token counts from assistant messages in the session JSONL.
async fn sum_session_tokens(path: &str) -> u64 {
    if !Path::new(path).exists() {
        return 0;
    }

    let file = match fs::File::open(path).await {
        Ok(f) => f,
        Err(_) => return 0,
    };

    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut total: u64 = 0;

    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(entry) = serde_json::from_str::<LogEntry>(&line) else {
            continue;
        };
        if entry.r#type != "assistant" {
            continue;
        }
        if let Some(msg) = entry.message {
            if msg.role == "assistant" {
                if let Some(usage) = msg.usage {
                    total += usage.input_tokens
                        + usage.output_tokens
                        + usage.cache_creation_input_tokens
                        + usage.cache_read_input_tokens;
                }
            }
        }
    }

    total
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_strategy ---

    #[test]
    fn strategy_block_variants() {
        assert_eq!(parse_strategy("block"), Ok(Strategy::Block));
        assert_eq!(parse_strategy("BLOCK"), Ok(Strategy::Block));
        assert_eq!(parse_strategy("Block"), Ok(Strategy::Block));
    }

    #[test]
    fn strategy_warn_variants() {
        assert_eq!(parse_strategy("warn"), Ok(Strategy::Warn));
        assert_eq!(parse_strategy("WARN"), Ok(Strategy::Warn));
        assert_eq!(parse_strategy("Warn"), Ok(Strategy::Warn));
    }

    #[test]
    fn strategy_whitespace_trimmed() {
        assert_eq!(parse_strategy("  warn  "), Ok(Strategy::Warn));
        assert_eq!(parse_strategy("  block  "), Ok(Strategy::Block));
    }

    #[test]
    fn strategy_rejects_unknown() {
        assert!(parse_strategy("").is_err());
        assert!(parse_strategy("skip").is_err());
        assert!(parse_strategy("warning").is_err());
        assert!(parse_strategy("1").is_err());
    }

    // --- parse_threshold ---

    #[test]
    fn parse_raw_integer() {
        assert_eq!(parse_threshold("100000"), Ok(100_000));
        assert_eq!(parse_threshold("0"), Ok(0));
        assert_eq!(parse_threshold("1"), Ok(1));
    }

    #[test]
    fn parse_k_suffix() {
        assert_eq!(parse_threshold("200K"), Ok(200_000));
        assert_eq!(parse_threshold("200k"), Ok(200_000));
        assert_eq!(parse_threshold("1K"), Ok(1_000));
    }

    #[test]
    fn parse_m_suffix() {
        assert_eq!(parse_threshold("1M"), Ok(1_000_000));
        assert_eq!(parse_threshold("1m"), Ok(1_000_000));
        assert_eq!(parse_threshold("2M"), Ok(2_000_000));
    }

    #[test]
    fn parse_whitespace_trimmed() {
        assert_eq!(parse_threshold("  50K  "), Ok(50_000));
    }

    #[test]
    fn parse_rejects_empty() {
        assert!(parse_threshold("").is_err());
        assert!(parse_threshold("   ").is_err());
    }

    #[test]
    fn parse_rejects_decimal() {
        assert!(parse_threshold("1.5M").is_err());
        assert!(parse_threshold("0.5K").is_err());
    }

    #[test]
    fn parse_rejects_garbage() {
        assert!(parse_threshold("abc").is_err());
        assert!(parse_threshold("1G").is_err()); // G not supported
        assert!(parse_threshold("K").is_err());
        assert!(parse_threshold("-100").is_err());
    }

    // --- Estimate ---

    #[test]
    fn prompt_tokens_heuristic() {
        // 40 bytes → 10 tokens
        let tokens = "a".repeat(40).len() as u64 / 4;
        assert_eq!(tokens, 10);
    }

    #[test]
    fn estimate_exceeds_threshold() {
        let est = Estimate {
            session_tokens: 90_000,
            prompt_tokens: 15_000,
            threshold: 100_000,
        };
        assert!(est.exceeds_threshold());
        assert_eq!(est.total(), 105_000);
    }

    #[test]
    fn estimate_within_threshold() {
        let est = Estimate {
            session_tokens: 80_000,
            prompt_tokens: 5_000,
            threshold: 100_000,
        };
        assert!(!est.exceeds_threshold());
    }

    #[tokio::test]
    async fn missing_transcript_returns_zero() {
        let tokens = sum_session_tokens("/nonexistent/path.jsonl").await;
        assert_eq!(tokens, 0);
    }
}
