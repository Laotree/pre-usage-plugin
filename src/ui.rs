use crate::estimator::Estimate;
use colored::Colorize;
use std::io::{self, BufRead, Write};

pub enum Choice {
    Send,
    Cancel,
}

/// Claude Code exit codes for UserPromptSubmit hooks.
///
/// | Code | Claude Code meaning          |
/// |------|------------------------------|
/// |  0   | Proceed — send the prompt    |
/// |  2   | Block  — discard the prompt  |
/// | else | Non-blocking error (proceeds)|
pub const EXIT_PROCEED: i32 = 0;
pub const EXIT_BLOCK: i32 = 2;

/// Render the token estimate to stderr.
pub fn render(est: &Estimate) {
    eprintln!();
    eprintln!(
        "  {}  session {} + prompt {} = {} tokens  (threshold {})",
        "⚠️  Token estimate".yellow().bold(),
        format_tokens(est.session_tokens).dimmed(),
        format_tokens(est.prompt_tokens).dimmed(),
        format_tokens(est.total()).yellow().bold(),
        format_tokens(est.threshold).dimmed(),
    );
    eprintln!();
}

/// Prompt the user for confirmation when the estimate exceeds the threshold.
/// Reads from /dev/tty so it works even when stdin is the hook JSON pipe.
/// If /dev/tty is unavailable, blocks the prompt by default (safe fallback).
pub fn confirm() -> Choice {
    let tty = match std::fs::File::open("/dev/tty") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("  pre-usage: cannot open /dev/tty ({e}) — blocking prompt by default.");
            std::process::exit(EXIT_BLOCK);
        }
    };

    eprint!("  [S]end  [C]ancel › ");
    io::stderr().flush().ok();

    let mut reader = io::BufReader::new(tty);
    let mut line = String::new();
    reader.read_line(&mut line).ok();

    match line.trim().to_ascii_lowercase().as_str() {
        "s" | "send" => Choice::Send,
        _ => Choice::Cancel,
    }
}

fn format_tokens(n: u64) -> String {
    // Insert thousands separators for readability
    let s = n.to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push('_');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_small() {
        assert_eq!(format_tokens(999), "999");
    }

    #[test]
    fn format_thousands() {
        assert_eq!(format_tokens(1_000), "1_000");
        assert_eq!(format_tokens(100_000), "100_000");
        assert_eq!(format_tokens(1_234_567), "1_234_567");
    }

    #[test]
    fn exit_code_constants() {
        assert_eq!(EXIT_PROCEED, 0);
        assert_eq!(EXIT_BLOCK, 2);
    }
}
