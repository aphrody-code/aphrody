// SPDX-License-Identifier: Apache-2.0
//
// Build script — emits compile-time env vars consumed by VersionCommand
// (commit sha, build epoch, target triple). No external deps; calls git
// directly. Falls back to "unknown" if git is missing or this is a tarball
// build (no .git tree).

use std::process::Command;

fn main() {
    let sha = Command::new("git")
        .args(["rev-parse", "--short=10", "HEAD"])
        .output()
        .ok()
        .and_then(|o| if o.status.success() { String::from_utf8(o.stdout).ok() } else { None })
        .map_or_else(|| "unknown".to_string(), |s| s.trim().to_string());
    println!("cargo:rustc-env=APHRODY_GIT_SHA={sha}");

    let unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    println!("cargo:rustc-env=APHRODY_BUILD_UNIX={unix}");

    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".into());
    println!("cargo:rustc-env=APHRODY_TARGET={target}");

    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".into());
    println!("cargo:rustc-env=APHRODY_PROFILE={profile}");

    // Rerun on next build when the commit changes or this script changes.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs");
}
