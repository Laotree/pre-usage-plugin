use std::io::{self, IsTerminal, Read};

mod estimator;
mod ui;

/// Hook stdin payload sent by Claude Code for UserPromptSubmit.
#[derive(serde::Deserialize)]
struct HookInput {
    #[serde(default)]
    transcript_path: String,
    #[serde(default)]
    prompt: String,
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

    let est = estimator::estimate(&input.transcript_path, &input.prompt).await;

    if !est.exceeds_threshold() {
        // Under threshold — silent pass.
        std::process::exit(0);
    }

    // Over threshold — behaviour depends on the configured strategy.
    ui::render(&est);

    match estimator::strategy() {
        estimator::Strategy::Warn => {
            // Warn mode: print the estimate (already done above) and auto-proceed.
            std::process::exit(0);
        }
        estimator::Strategy::Block => {
            // Block mode: ask the user to confirm before sending.
            match ui::confirm() {
                ui::Choice::Send => std::process::exit(ui::EXIT_PROCEED),
                ui::Choice::Cancel => {
                    eprintln!("  Prompt cancelled.");
                    std::process::exit(ui::EXIT_BLOCK);
                }
            }
        }
    }
}
