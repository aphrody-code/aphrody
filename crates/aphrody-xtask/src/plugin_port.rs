// plugin_port.rs — Rust port of scripts/plugin-contract-port.ts.
//
// Vendors the upstream `@openclaw/plugin-package-contract` TypeScript module
// as `packages/plugin-package-contract/index.ts` inside aphrody, with a
// canonical SPDX header, upstream attribution comments, and a companion
// `README.md` documenting the contract surface + link back to the source
// path. The vendored copy is byte-faithful to the original interface
// declarations and helper functions.
//
// Usage:
//     cargo xtask plugin-port
//     cargo xtask plugin-port --check
//     cargo xtask plugin-port --source-oc <path>
//
// Exit codes:
//     0  vendored file + README written successfully
//     1  --check requested and upstream source unreadable
//     2  CLI / IO error

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use clap::Args as ClapArgs;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Exit 1 if the upstream openclaw checkout cannot be located.
    #[arg(long)]
    check: bool,

    /// Override path to the openclaw clone (default: C:/src/openclaw).
    #[arg(long = "source-oc", default_value = "C:/src/openclaw")]
    source_oc: PathBuf,

    /// Override path to the open-design clone (default: C:/src/open-design).
    /// Kept for CLI parity with the .ts script even though only `source-oc` is
    /// consulted by the vendoring pipeline today.
    #[arg(long = "source-od", default_value = "C:/src/open-design")]
    #[allow(dead_code)]
    source_od: PathBuf,
}

const SPDX: &str = "// SPDX-License-Identifier: Apache-2.0";

pub(crate) fn run(args: Args) -> Result<()> {
    let upstream_ts = args
        .source_oc
        .join("packages")
        .join("plugin-package-contract")
        .join("src")
        .join("index.ts");
    let upstream_pkg = args
        .source_oc
        .join("packages")
        .join("plugin-package-contract")
        .join("package.json");

    if !upstream_ts.exists() || !upstream_pkg.exists() {
        let msg = format!(
            "upstream contract not found at {} (need {})",
            args.source_oc.display(),
            upstream_ts.display(),
        );
        if args.check {
            eprintln!("[plugin-contract-port] check failed: {msg}");
            std::process::exit(1);
        }
        eprintln!("[plugin-contract-port] error: {msg}");
        std::process::exit(2);
    }

    let repo_root = repo_root()?;
    let out_dir = repo_root
        .join("packages")
        .join("plugin-package-contract");
    let out_ts = out_dir.join("index.ts");
    let out_readme = out_dir.join("README.md");
    let out_pkg = out_dir.join("package.json");

    fs::create_dir_all(&out_dir).context("mkdir vendor dir")?;

    let upstream_src = fs::read_to_string(&upstream_ts).context("read upstream index.ts")?;
    fs::write(&out_ts, render_vendored(&upstream_src)).context("write vendored index.ts")?;
    fs::write(&out_readme, render_readme(&upstream_ts)).context("write README.md")?;
    fs::write(&out_pkg, render_package_json()).context("write package.json")?;

    for p in [&out_ts, &out_readme, &out_pkg] {
        println!("wrote {}", p.display());
    }
    println!("vendored {} bytes from {}", upstream_src.len(), upstream_ts.display());
    Ok(())
}

fn render_vendored(source: &str) -> String {
    let banner = format!("{SPDX}
//
// Vendored from upstream openclaw/openclaw
//   path:    packages/plugin-package-contract/src/index.ts
//   project: https://github.com/openclaw/openclaw
//   licence: MIT (Copyright (c) 2025 Peter Steinberger)
//
// Re-published in aphrody under Apache-2.0 with explicit attribution. The
// upstream MIT licence is compatible with Apache-2.0 redistribution; the
// original copyright notice above must be preserved per the MIT terms.
//
// DO NOT EDIT BY HAND \u{2014} regenerate from upstream instead.
//     cargo xtask plugin-port
// ----------------------------------------------------------------------------
");
    format!("{banner}\n{}\n", source.trim_end())
}

fn render_readme(upstream_path: &Path) -> String {
    let path = upstream_path.to_string_lossy().replace('\\', "/");
    format!(
        "<!-- SPDX-License-Identifier: Apache-2.0 -->

# @aphrody/plugin-package-contract

In-tree mirror of [openclaw/openclaw](https://github.com/openclaw/openclaw)'s
`@openclaw/plugin-package-contract` package, vendored at
`{path}`.

## Refresh

```bash
cargo xtask plugin-port
```

Pass `--source-oc <path>` to override the upstream clone location
(defaults to `C:/src/openclaw`).

Use `--check` in CI to fail-fast when the upstream clone is missing.
"
    )
}

fn render_package_json() -> String {
    let value = serde_json::json!({
        "name": "@aphrody/plugin-package-contract",
        "version": "0.0.0-vendored",
        "private": true,
        "type": "module",
        "description": "Vendored mirror of @openclaw/plugin-package-contract.",
        "license": "Apache-2.0",
        "exports": { ".": "./index.ts" },
        "aphrody": {
            "vendoredFrom": "openclaw/openclaw#packages/plugin-package-contract",
            "upstreamLicense": "MIT",
            "upstreamCopyright": "Copyright (c) 2025 Peter Steinberger",
            "refreshScript": "cargo xtask plugin-port"
        }
    });
    format!("{}\n", serde_json::to_string_pretty(&value).unwrap())
}

fn repo_root() -> Result<PathBuf> {
    if let Ok(dir) = env::var("CARGO_WORKSPACE_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("spawn git rev-parse")?;
    if out.status.success() {
        return Ok(PathBuf::from(String::from_utf8(out.stdout)?.trim()));
    }
    Ok(env::current_dir()?)
}
