use crate::estimator::Estimate;
use colored::Colorize;
use std::io::{self, BufRead, Write};

pub enum Choice {
    Send,
    Cancel,
}

/// Render the token estimate to stderr and return whether we are over the threshold.
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
pub fn confirm() -> Choice {
    eprint!("  [S]end  [C]ancel › ");
    io::stderr().flush().ok();

    let tty = std::fs::File::open("/dev/tty")
        .expect("cannot open /dev/tty — interactive prompt unavailable");
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
}
