use serde::Deserialize;
use std::path::Path;

/// Per-project configuration loaded from `.claude/pre-usage.toml`.
#[derive(Debug, Default, Deserialize)]
pub struct ProjectConfig {
    pub threshold: Option<String>,
    pub strategy: Option<String>,
}

/// Load project-level config from `{cwd}/.claude/pre-usage.toml`.
///
/// Returns `None` if the file doesn't exist or can't be parsed.
pub fn load(cwd: &str) -> Option<ProjectConfig> {
    let path = Path::new(cwd).join(".claude").join("pre-usage.toml");
    if !path.exists() {
        return None;
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "pre-usage: warning: could not read {}: {e}",
                path.display()
            );
            return None;
        }
    };
    match toml::from_str(&content) {
        Ok(cfg) => Some(cfg),
        Err(e) => {
            eprintln!(
                "pre-usage: warning: could not parse {}: {e}",
                path.display()
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_config() {
        let toml_str = r#"
threshold = "100K"
strategy = "block"
"#;
        let cfg: ProjectConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.threshold.as_deref(), Some("100K"));
        assert_eq!(cfg.strategy.as_deref(), Some("block"));
    }

    #[test]
    fn parse_partial_config() {
        let toml_str = r#"
strategy = "warn"
"#;
        let cfg: ProjectConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.threshold, None);
        assert_eq!(cfg.strategy.as_deref(), Some("warn"));
    }

    #[test]
    fn parse_empty_config() {
        let toml_str = "";
        let cfg: ProjectConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.threshold, None);
        assert_eq!(cfg.strategy, None);
    }
}
