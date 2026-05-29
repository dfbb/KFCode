//! Injects the build date into the binary as the `KFCODE_BUILD_DATE` env var,
//! formatted `YYYY.MM.DD`. Honors `SOURCE_DATE_EPOCH` for reproducible builds.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn main() {
    // Re-run if the reproducible-build timestamp changes.
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");

    let now = match std::env::var("SOURCE_DATE_EPOCH") {
        Ok(epoch) => match epoch.trim().parse::<u64>() {
            Ok(secs) => UNIX_EPOCH + Duration::from_secs(secs),
            Err(_) => SystemTime::now(),
        },
        Err(_) => SystemTime::now(),
    };

    let date = chrono::DateTime::<chrono::Utc>::from(now)
        .format("%Y.%m.%d")
        .to_string();

    println!("cargo:rustc-env=KFCODE_BUILD_DATE={date}");
}
