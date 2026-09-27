//! The application version, set at build time by the release pipeline.

/// The version shown in the app. The release pipeline sets
/// `LIVESPLIT_ASR_BRIDGE_VERSION` at build time; local builds show the
/// `Cargo.toml` version, `0.0.0-dev`. Cargo rebuilds automatically when the
/// variable changes, because `option_env!` is tracked.
pub const VERSION: &str = resolve(
    option_env!("LIVESPLIT_ASR_BRIDGE_VERSION"),
    env!("CARGO_PKG_VERSION"),
);

/// Picks the build-time version when it is set and not empty, otherwise the
/// fallback.
pub const fn resolve(build: Option<&'static str>, fallback: &'static str) -> &'static str {
    match build {
        Some(version) if !version.is_empty() => version,
        _ => fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_build_version_when_set() {
        assert_eq!(resolve(Some("1.2.3-rc.4"), "0.0.0-dev"), "1.2.3-rc.4");
    }

    #[test]
    fn falls_back_when_unset() {
        assert_eq!(resolve(None, "0.0.0-dev"), "0.0.0-dev");
    }

    #[test]
    fn falls_back_when_set_but_empty() {
        assert_eq!(resolve(Some(""), "0.0.0-dev"), "0.0.0-dev");
    }

    #[test]
    fn version_is_never_empty() {
        assert!(!VERSION.is_empty());
    }
}
