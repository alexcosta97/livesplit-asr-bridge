//! Embeds the app icon and version information in the Windows `.exe`, so
//! Explorer and the file properties show them.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        windows::embed_resources();
    }
}

#[cfg(windows)]
mod windows {
    use std::env;

    /// The display name, as in `ui::DISPLAY_NAME`.
    const DISPLAY_NAME: &str = "LiveSplit One ASR Bridge";
    const ICON: &str = "assets/brand/icons/icon.ico";

    pub fn embed_resources() {
        println!("cargo:rerun-if-changed={ICON}");
        println!("cargo:rerun-if-env-changed=LIVESPLIT_ASR_BRIDGE_VERSION");

        // The same version the app shows (see `src/version.rs`).
        let version = env::var("LIVESPLIT_ASR_BRIDGE_VERSION")
            .ok()
            .filter(|version| !version.is_empty())
            .unwrap_or_else(|| env::var("CARGO_PKG_VERSION").unwrap());
        let numeric = numeric_version(&version);

        winresource::WindowsResource::new()
            .set_icon(ICON)
            .set("FileDescription", DISPLAY_NAME)
            .set("ProductName", DISPLAY_NAME)
            .set("FileVersion", &version)
            .set("ProductVersion", &version)
            .set_version_info(winresource::VersionInfo::FILEVERSION, numeric)
            .set_version_info(winresource::VersionInfo::PRODUCTVERSION, numeric)
            .compile()
            .expect("the Windows resources compile");
    }

    /// Packs the `X.Y.Z` part of a version, such as `0.4.0-rc.2`, into the
    /// numeric form Windows expects, `X.Y.Z.0`.
    fn numeric_version(version: &str) -> u64 {
        let core = version.split(['-', '+']).next().unwrap_or_default();
        core.split('.')
            .take(3)
            .zip([48, 32, 16])
            .map(|(part, shift)| part.parse::<u64>().unwrap_or(0) << shift)
            .sum()
    }
}
