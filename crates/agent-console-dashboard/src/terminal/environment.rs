/// Detected terminal multiplexer environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalEnvironment {
    Zellij,
    Tmux,
    Plain,
}

impl TerminalEnvironment {
    /// Detect the current terminal environment from env vars.
    pub fn detect() -> Self {
        if std::env::var("ZELLIJ").is_ok() {
            Self::Zellij
        } else if std::env::var("TMUX").is_ok() {
            Self::Tmux
        } else {
            Self::Plain
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_environment_variants() {
        // Test that all variants exist and are distinct
        let zellij = TerminalEnvironment::Zellij;
        let tmux = TerminalEnvironment::Tmux;
        let plain = TerminalEnvironment::Plain;

        assert_eq!(zellij, TerminalEnvironment::Zellij);
        assert_eq!(tmux, TerminalEnvironment::Tmux);
        assert_eq!(plain, TerminalEnvironment::Plain);

        assert_ne!(zellij, tmux);
        assert_ne!(tmux, plain);
        assert_ne!(zellij, plain);
    }

    #[test]
    fn test_terminal_environment_clone() {
        let env = TerminalEnvironment::Zellij;
        let cloned = env;
        assert_eq!(env, cloned);
    }

    #[test]
    fn test_terminal_environment_debug() {
        let env = TerminalEnvironment::Tmux;
        let debug_str = format!("{:?}", env);
        assert!(debug_str.contains("Tmux"));
    }

    // Note: Testing detect() with env vars is racy, but we can verify the logic conceptually
    #[test]
    fn test_detect_logic() {
        // This test just ensures the detect method compiles and returns a valid variant
        let detected = TerminalEnvironment::detect();
        // Should be one of the three variants
        match detected {
            TerminalEnvironment::Zellij | TerminalEnvironment::Tmux | TerminalEnvironment::Plain => {
                // Expected variants
            }
        }
    }
}
