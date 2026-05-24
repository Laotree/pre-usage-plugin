use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

mod config;
mod estimator;
mod ui;

/// Hook stdin payload sent by Claude Code for UserPromptSubmit.
#[derive(serde::Deserialize)]
struct HookInput {
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    transcript_path: String,
    #[serde(default)]
    prompt: String,
    #[serde(default)]
    cwd: String,
}

/// Path to the skip marker for a session.
fn skip_marker_path(session_id: &str) -> PathBuf {
    std::env::temp_dir().join(format!("pre-usage-skip-{session_id}"))
}

/// Check whether the user chose to skip blocking for this session.
fn is_session_skipped(session_id: &str) -> bool {
    if session_id.is_empty() {
        return false;
    }
    skip_marker_path(session_id).exists()
}

/// Write a marker so subsequent prompts in this session auto-proceed.
fn mark_session_skipped(session_id: &str) {
    if session_id.is_empty() {
        return;
    }
    let path = skip_marker_path(session_id);
    let _ = std::fs::write(&path, b"");
}

#[tokio::main]
async fn main() {
    if std::io::stderr().is_terminal() {
        colored::control::set_override(true);
    }

    // Read the full hook JSON from stdin.
    let mut raw = String::new();
    io::stdin().read_to_string(&mut raw).unwrap_or(0);

    let input: HookInput = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("pre-usage: failed to parse hook input: {e}");
            std::process::exit(0); // don't block Claude on a parse error
        }
    };

    let cfg = estimator::resolve_config(&input.cwd);
    let est = estimator::estimate(&input.transcript_path, &input.prompt, cfg.threshold).await;

    if !est.exceeds_threshold() {
        // Under threshold — silent pass.
        std::process::exit(0);
    }

    // Over threshold — behaviour depends on the resolved strategy.
    ui::render(&est);

    // If the user chose to skip blocking earlier in this session,
    // auto-proceed (warn mode) regardless of the configured strategy.
    if is_session_skipped(&input.session_id) {
        std::process::exit(0);
    }

    match cfg.strategy {
        estimator::Strategy::Warn => {
            // Warn mode: print the estimate (already done above) and auto-proceed.
            std::process::exit(0);
        }
        estimator::Strategy::Block => {
            // Block mode: ask the user to confirm before sending.
            match ui::confirm(&est) {
                ui::Choice::Send => std::process::exit(ui::EXIT_PROCEED),
                ui::Choice::SkipSession => {
                    mark_session_skipped(&input.session_id);
                    eprintln!("  Session will auto-proceed for remaining prompts.");
                    std::process::exit(ui::EXIT_PROCEED);
                }
                ui::Choice::Cancel => {
                    eprintln!("  Prompt cancelled.");
                    std::process::exit(ui::EXIT_BLOCK);
                }
            }
        }
    }
}
