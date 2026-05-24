use serde::Deserialize;
use std::path::Path;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};

const DEFAULT_THRESHOLD: u64 = 100_000;

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

/// Read `PRE_USAGE_THRESHOLD` from the environment (default 100 000).
pub fn threshold() -> u64 {
    std::env::var("PRE_USAGE_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_THRESHOLD)
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
