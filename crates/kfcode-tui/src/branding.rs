//! Application branding constants (name, short name, version, tagline).

/// Full application name displayed in the UI.
pub const APP_NAME: &str = "KFCode";
/// Abbreviated application name used in terminal title strings.
pub const APP_SHORT_NAME: &str = "KFCode";
/// Package version (from Cargo.toml `[workspace.package].version`), e.g. `0.1.1`.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Build date in `YYYY.MM.DD`, injected by build.rs.
pub const APP_BUILD_DATE: &str = env!("KFCODE_BUILD_DATE");
/// Combined "<version> <build-date>" string shown in the about/status view.
pub const APP_VERSION_DATE: &str = concat!(env!("CARGO_PKG_VERSION"), " ", env!("KFCODE_BUILD_DATE"));
/// Marketing tagline shown on the home screen.
pub const APP_TAGLINE: &str = "A Rusted KFCode Version";

