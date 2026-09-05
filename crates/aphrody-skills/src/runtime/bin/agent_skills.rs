// SPDX-License-Identifier: Apache-2.0
//
// `agent-skills` — Rust port of the @aphrody/skills CLI.
//
// Subcommands (matrix vs TS):
//   list       → flat table OR JSON of discovered skills, either across
//                active sources or under an explicit <PATH>.
//   validate   → enforce frontmatter contract (name/description/kebab).
//   to-prompt  → emit the SKILL.md body of one local skill (for prompt
//                injection by an orchestrator).
//   info       → print the full SKILL.md.
//   where      → print absolute path(s) of one named skill.
//   sources    → print the source registry with discovered counts.
//   install    → copy one SKILL.md into $APHRODY_SKILLS_HOME.
//   sync       → mirror discovered SKILL.md files into $APHRODY_SKILLS_HOME.
//                When invoked WITH a <owner>/<repo> spec, delegates to
//                `cargo xtask skills-sync` so the existing clone-from-GitHub
//                code path stays the single source of truth.
//   remove     → uninstall a skill from $APHRODY_SKILLS_HOME and drop its
//                manifest entry (symmetric inverse of `install`).

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Serialize;

use aphrody_skills::runtime::{
    active_sources, discover::discover_skills_in_path, discover_agent_skills, discover_plugin_skills,
    discover_skills, ensure_home_exists, manifest::ManifestEntry, parse_frontmatter, read_manifest,
    resolve_source_root, skills_home, source_by_slug, sources::SourceSlug, upsert_entry,
    validate_frontmatter, write_manifest, SourceSpec, SOURCES,
};

#[derive(Debug, Parser)]
#[command(
    name = "agent-skills",
    version,
    about = "Unified SKILL.md aggregator CLI (Rust port of @aphrody/skills)."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Flat table of every discovered skill.
    List(ListArgs),
    /// Validate frontmatter against the required contract.
    Validate(ListArgs),
    /// Emit the SKILL.md body for one skill (for prompt injection).
    #[command(name = "to-prompt")]
    ToPrompt(ToPromptArgs),
    /// Print the full SKILL.md for one skill.
    Info(InfoArgs),
    /// Print the absolute path(s) of one named skill.
    Where(InfoArgs),
    /// Print the source registry with discovered counts.
    Sources(SourcesArgs),
    /// Copy one SKILL.md into $APHRODY_SKILLS_HOME.
    Install(InfoArgs),
    /// Mirror SKILL.md files (locally or from a GitHub repo).
    Sync(SyncArgs),
    /// Uninstall a skill from $APHRODY_SKILLS_HOME (inverse of `install`).
    #[command(visible_alias = "rm", alias = "uninstall")]
    Remove(InfoArgs),
    /// Search discovered skills by keyword (name + description, scriptable).
    Find(FindArgs),
    /// Scaffold a new SKILL.md template in a directory.
    Init(InitArgs),
    /// Refresh installed skills from their recorded source path.
    Update(UpdateArgs),
}

#[derive(Debug, clap::Args)]
struct ListArgs {
    /// Optional path to a local skills directory. If absent, scans every
    /// active source from the registry.
    path: Option<PathBuf>,
    /// Filter to a single source slug (registry mode only).
    #[arg(long)]
    source: Option<String>,
    /// Machine-readable JSON output.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, clap::Args)]
struct ToPromptArgs {
    /// Path to one SKILL.md OR to its containing dir.
    target: PathBuf,
}

#[derive(Debug, clap::Args)]
struct InfoArgs {
    /// Skill name (must match the directory name).
    name: String,
    /// Filter to a single source slug.
    #[arg(long)]
    source: Option<String>,
}

#[derive(Debug, clap::Args)]
struct SourcesArgs {
    #[arg(long)]
    json: bool,
}

#[derive(Debug, clap::Args)]
struct SyncArgs {
    /// Optional GitHub source as `<owner>/<repo>[#<ref>]`. When set,
    /// delegates to `cargo xtask skills-sync`.
    repo: Option<String>,
    /// Source slug filter (local mirror mode only).
    #[arg(long)]
    source: Option<String>,
    /// Overwrite existing skills (forwarded to `cargo xtask`).
    #[arg(long)]
    force: bool,
    /// Show what would happen without writing (local mirror mode only).
    #[arg(long = "dry-run")]
    dry_run: bool,
}

#[derive(Debug, clap::Args)]
struct FindArgs {
    /// Keyword matched (case-insensitive) against skill name + description.
    /// Omit to list everything discovered.
    query: Option<String>,
    /// Machine-readable JSON output.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, clap::Args)]
struct InitArgs {
    /// Optional subdirectory to scaffold into; defaults to the current dir.
    name: Option<String>,
}

#[derive(Debug, clap::Args)]
struct UpdateArgs {
    /// Specific installed skills to refresh; default = every installed skill.
    names: Vec<String>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let res = match cli.cmd.unwrap_or(Cmd::List(ListArgs {
        path: None,
        source: None,
        json: false,
    })) {
        Cmd::List(a) => run_list(&a),
        Cmd::Validate(a) => run_validate(&a),
        Cmd::ToPrompt(a) => run_to_prompt(&a),
        Cmd::Info(a) => run_info(&a),
        Cmd::Where(a) => run_where(&a),
        Cmd::Sources(a) => run_sources(&a),
        Cmd::Install(a) => run_install(&a),
        Cmd::Sync(a) => run_sync(&a),
        Cmd::Remove(a) => run_remove(&a),
        Cmd::Find(a) => run_find(&a),
        Cmd::Init(a) => run_init(&a),
        Cmd::Update(a) => run_update(&a),
    };
    match res {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            let _ = writeln!(std::io::stderr(), "agent-skills: {err:#}");
            ExitCode::from(1)
        }
    }
}

// ---------------------------------------------------------------------------
// list / validate share the same row-collection pipeline.
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct ListRow {
    name: String,
    source: String,
    schema: String,
    mode: String,
    description: String,
    loc: usize,
    size_bytes: u64,
    path: String,
    /// Hidden unless `INSTALL_INTERNAL_SKILLS=1` (frontmatter `metadata.internal`).
    #[serde(skip)]
    internal: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<String>,
}

fn collect_rows(args: &ListArgs) -> Result<Vec<ListRow>> {
    let mut rows: Vec<ListRow> = Vec::new();
    if let Some(p) = &args.path {
        let slug = args
            .source
            .as_deref()
            .and_then(SourceSlug::from_str)
            .unwrap_or(SourceSlug::ClaudeCode);
        let hits = discover_skills_in_path(p, slug);
        for h in hits {
            rows.push(row_from(&h.skill_md_path, slug.as_str(), "aphrody", &h.name)?);
        }
    } else {
        let slug_filter = args.source.as_deref();
        let sources: Vec<&SourceSpec> = if let Some(s) = slug_filter {
            source_by_slug(s).into_iter().collect()
        } else {
            active_sources()
        };
        for spec in sources {
            for h in discover_skills(spec) {
                rows.push(row_from(
                    &h.skill_md_path,
                    spec.slug.as_str(),
                    spec.schema.as_str(),
                    &h.name,
                )?);
            }
        }
        // Plugin-manifest + agent-dir discovery (claude-code / gemini-cli /
        // antigravity / agy), in addition to the upstream catalog registry.
        let extra = discover_plugin_skills(&plugin_bases())
            .into_iter()
            .chain(discover_agent_skills());
        for h in extra {
            if slug_filter.is_none_or(|s| s == h.source.as_str()) {
                rows.push(row_from(&h.skill_md_path, h.source.as_str(), "aphrody", &h.name)?);
            }
        }
    }
    // Drop duplicate SKILL.md paths surfaced by more than one discovery root.
    let mut seen_paths = std::collections::HashSet::new();
    rows.retain(|r| seen_paths.insert(r.path.clone()));
    // Internal skills stay hidden unless explicitly opted in.
    if !install_internal() {
        rows.retain(|r| !r.internal);
    }
    rows.sort_by(|a, b| {
        if a.source == b.source {
            a.name.cmp(&b.name)
        } else {
            a.source.cmp(&b.source)
        }
    });
    Ok(rows)
}

fn row_from(skill_md: &Path, source: &str, schema: &str, name: &str) -> Result<ListRow> {
    let bytes = fs::read(skill_md).unwrap_or_default();
    let size = bytes.len() as u64;
    let text = String::from_utf8_lossy(&bytes);
    let loc = text.lines().count();
    let fm = parse_frontmatter(&text);
    let mode = fm
        .od
        .as_ref()
        .and_then(|o| o.mode.clone())
        .unwrap_or_default();
    let description = fm.description.clone().unwrap_or_default();
    let internal = fm.metadata.get("internal").is_some_and(|v| v == "true");
    Ok(ListRow {
        name: name.to_string(),
        source: source.to_string(),
        schema: schema.to_string(),
        mode,
        description,
        loc,
        size_bytes: size,
        path: skill_md.to_string_lossy().into_owned(),
        internal,
        errors: Vec::new(),
    })
}

fn run_list(args: &ListArgs) -> Result<u8> {
    let rows = collect_rows(args)?;
    let mut stdout = std::io::stdout().lock();
    if args.json {
        writeln!(stdout, "{}", serde_json::to_string_pretty(&rows)?)?;
        return Ok(0);
    }
    if rows.is_empty() {
        writeln!(
            stdout,
            "No skills discovered. Run `agent-skills sources` to see source status."
        )?;
        return Ok(0);
    }
    let name_w = rows.iter().map(|r| r.name.len()).max().unwrap_or(4).max(4);
    let src_w = rows.iter().map(|r| r.source.len()).max().unwrap_or(6).max(6);
    let mode_w = rows.iter().map(|r| r.mode.len()).max().unwrap_or(4).max(4);
    writeln!(
        stdout,
        "{:<name_w$}  {:<src_w$}  {:<mode_w$}  {:>5}  DESCRIPTION",
        "NAME",
        "SOURCE",
        "MODE",
        "LOC",
        name_w = name_w,
        src_w = src_w,
        mode_w = mode_w
    )?;
    let total_w = name_w + src_w + mode_w + 5 + 13 + 12;
    writeln!(stdout, "{}", "-".repeat(total_w))?;
    for r in &rows {
        writeln!(
            stdout,
            "{:<name_w$}  {:<src_w$}  {:<mode_w$}  {:>5}  {}",
            r.name,
            r.source,
            r.mode,
            r.loc,
            truncate(&r.description, 72),
            name_w = name_w,
            src_w = src_w,
            mode_w = mode_w
        )?;
    }
    writeln!(stdout, "\n{} skill(s).", rows.len())?;
    Ok(0)
}

fn run_validate(args: &ListArgs) -> Result<u8> {
    let mut rows = collect_rows(args)?;
    let mut total_errs = 0usize;
    for row in &mut rows {
        let text = fs::read_to_string(&row.path).unwrap_or_default();
        let fm = parse_frontmatter(&text);
        let errs = validate_frontmatter(&fm, Some(&row.name));
        if !errs.is_empty() {
            total_errs += errs.len();
            row.errors = errs.iter().map(ToString::to_string).collect();
        }
    }
    let mut stdout = std::io::stdout().lock();
    if args.json {
        writeln!(stdout, "{}", serde_json::to_string_pretty(&rows)?)?;
    } else {
        for row in &rows {
            if row.errors.is_empty() {
                writeln!(stdout, "ok    {}/{}", row.source, row.name)?;
            } else {
                for e in &row.errors {
                    writeln!(stdout, "FAIL  {}/{}  {}", row.source, row.name, e)?;
                }
            }
        }
        writeln!(
            stdout,
            "\n{} skill(s) — {} error(s).",
            rows.len(),
            total_errs
        )?;
    }
    if total_errs > 0 {
        Ok(1)
    } else {
        Ok(0)
    }
}

fn run_to_prompt(args: &ToPromptArgs) -> Result<u8> {
    let p = if args.target.is_dir() {
        args.target.join("SKILL.md")
    } else {
        args.target.clone()
    };
    let body = fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))?;
    let (_fm, content) = aphrody_skills::runtime::split_frontmatter(&body);
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(content.as_bytes())?;
    if !content.ends_with('\n') {
        writeln!(stdout)?;
    }
    Ok(0)
}

fn find_in_registry(name: &str, source: Option<&str>) -> Vec<(SourceSlug, PathBuf)> {
    let mut hits = Vec::new();
    for spec in active_sources() {
        if let Some(s) = source {
            if spec.slug.as_str() != s {
                continue;
            }
        }
        for sk in discover_skills(spec) {
            if sk.name == name {
                hits.push((spec.slug, sk.skill_md_path));
            }
        }
    }
    hits
}

fn run_info(args: &InfoArgs) -> Result<u8> {
    let hits = find_in_registry(&args.name, args.source.as_deref());
    if hits.is_empty() {
        anyhow::bail!("no skill named '{}' found", args.name);
    }
    if hits.len() > 1 {
        let srcs: Vec<&str> = hits.iter().map(|(s, _)| s.as_str()).collect();
        anyhow::bail!(
            "'{}' is ambiguous across sources ({}). Re-run with --source=<slug>.",
            args.name,
            srcs.join(", ")
        );
    }
    let (slug, path) = &hits[0];
    let body = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "# source: {}", slug.as_str())?;
    writeln!(stdout, "# path:   {}", path.display())?;
    writeln!(stdout)?;
    stdout.write_all(body.as_bytes())?;
    if !body.ends_with('\n') {
        writeln!(stdout)?;
    }
    Ok(0)
}

fn run_where(args: &InfoArgs) -> Result<u8> {
    let hits = find_in_registry(&args.name, args.source.as_deref());
    if hits.is_empty() {
        anyhow::bail!("no skill named '{}' found", args.name);
    }
    let mut stdout = std::io::stdout().lock();
    for (slug, path) in hits {
        writeln!(stdout, "{}\t{}", slug.as_str(), path.display())?;
    }
    Ok(0)
}

fn run_sources(args: &SourcesArgs) -> Result<u8> {
    #[derive(Serialize)]
    struct Row {
        slug: String,
        label: String,
        schema: String,
        path: String,
        count: usize,
        status: String,
    }
    let mut rows = Vec::new();
    for spec in SOURCES {
        let root = resolve_source_root(spec);
        let skills = discover_skills(spec);
        let status = if root.is_none() {
            "missing"
        } else if skills.is_empty() {
            "empty"
        } else {
            "ok"
        };
        rows.push(Row {
            slug: spec.slug.as_str().to_string(),
            label: spec.label.to_string(),
            schema: spec.schema.as_str().to_string(),
            path: root
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| "(missing)".to_string()),
            count: skills.len(),
            status: status.to_string(),
        });
    }
    let mut stdout = std::io::stdout().lock();
    if args.json {
        writeln!(stdout, "{}", serde_json::to_string_pretty(&rows)?)?;
        return Ok(0);
    }
    let slug_w = rows.iter().map(|r| r.slug.len()).max().unwrap_or(4).max(4);
    let schema_w = rows
        .iter()
        .map(|r| r.schema.len())
        .max()
        .unwrap_or(6)
        .max(6);
    writeln!(
        stdout,
        "{:<slug_w$}  {:<schema_w$}  {:>6}  STATUS   PATH",
        "SLUG",
        "SCHEMA",
        "SKILLS",
        slug_w = slug_w,
        schema_w = schema_w
    )?;
    writeln!(stdout, "{}", "-".repeat(slug_w + schema_w + 6 + 8 + 8))?;
    for r in &rows {
        writeln!(
            stdout,
            "{:<slug_w$}  {:<schema_w$}  {:>6}  {:<7}  {}",
            r.slug,
            r.schema,
            r.count,
            r.status,
            r.path,
            slug_w = slug_w,
            schema_w = schema_w
        )?;
    }
    Ok(0)
}

fn run_install(args: &InfoArgs) -> Result<u8> {
    let hits = find_in_registry(&args.name, args.source.as_deref());
    if hits.is_empty() {
        anyhow::bail!("no skill named '{}' found", args.name);
    }
    if hits.len() > 1 {
        let srcs: Vec<&str> = hits.iter().map(|(s, _)| s.as_str()).collect();
        anyhow::bail!(
            "'{}' is ambiguous ({}). Re-run with --source=<slug>.",
            args.name,
            srcs.join(", ")
        );
    }
    let (slug, src_path) = &hits[0];
    let home = ensure_home_exists()?;
    let target_dir = home.join(slug.as_str()).join(&args.name);
    fs::create_dir_all(&target_dir)
        .with_context(|| format!("mkdir {}", target_dir.display()))?;
    let target = target_dir.join("SKILL.md");
    fs::copy(src_path, &target)
        .with_context(|| format!("copy {} -> {}", src_path.display(), target.display()))?;
    let size = fs::metadata(src_path).map(|m| m.len()).unwrap_or(0);
    upsert_entry(ManifestEntry {
        name: args.name.clone(),
        source: *slug,
        source_path: src_path.to_string_lossy().into_owned(),
        copied_at: chrono_like_now(),
        size_bytes: size,
    })?;
    writeln!(
        std::io::stdout(),
        "installed {}/{} -> {} ({} bytes)",
        slug.as_str(),
        args.name,
        target.display(),
        size
    )?;
    Ok(0)
}

fn run_remove(args: &InfoArgs) -> Result<u8> {
    let mut manifest = read_manifest()?;
    let wanted = args.source.as_deref().and_then(SourceSlug::from_str);
    let matches: Vec<usize> = manifest
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.name == args.name && wanted.is_none_or(|s| e.source == s))
        .map(|(i, _)| i)
        .collect();
    if matches.is_empty() {
        anyhow::bail!(
            "no installed skill named '{}' (run `agent-skills list`)",
            args.name
        );
    }
    if matches.len() > 1 {
        let srcs: Vec<&str> = matches
            .iter()
            .map(|&i| manifest.entries[i].source.as_str())
            .collect();
        anyhow::bail!(
            "'{}' is installed from multiple sources ({}). Re-run with --source=<slug>.",
            args.name,
            srcs.join(", ")
        );
    }
    let idx = matches[0];
    let slug = manifest.entries[idx].source;
    let dir = skills_home().join(slug.as_str()).join(&args.name);
    if dir.exists() {
        fs::remove_dir_all(&dir).with_context(|| format!("remove {}", dir.display()))?;
    }
    manifest.entries.remove(idx);
    write_manifest(&manifest)?;
    writeln!(
        std::io::stdout(),
        "removed {}/{} ({})",
        slug.as_str(),
        args.name,
        dir.display()
    )?;
    Ok(0)
}

fn run_sync(args: &SyncArgs) -> Result<u8> {
    // If a GitHub source is provided, delegate to the existing
    // `cargo xtask skills-sync` binary so the clone-from-github code path
    // stays the single source of truth (no duplication).
    if let Some(repo) = &args.repo {
        let mut cmd = Command::new("cargo");
        cmd.args(["xtask", "skills-sync", repo]);
        if args.force {
            cmd.arg("--force");
        }
        let status = cmd
            .status()
            .with_context(|| "spawn `cargo xtask skills-sync`")?;
        return Ok(status.code().unwrap_or(1) as u8);
    }

    // Local mirror mode: copy every discovered SKILL.md into the home dir.
    let home = if args.dry_run {
        aphrody_skills::runtime::skills_home()
    } else {
        ensure_home_exists()?
    };
    let sources: Vec<&SourceSpec> = if let Some(s) = &args.source {
        source_by_slug(s).into_iter().collect()
    } else {
        active_sources()
    };
    if sources.is_empty() {
        anyhow::bail!("no source matches '{}'", args.source.as_deref().unwrap_or("<all>"));
    }
    let mut stdout = std::io::stdout().lock();
    writeln!(
        stdout,
        "sync target: {}{}",
        home.display(),
        if args.dry_run { "  (dry-run)" } else { "" }
    )?;
    let mut total = 0usize;
    let mut total_bytes = 0u64;
    for spec in sources {
        let mut copied = 0usize;
        let mut bytes = 0u64;
        for sk in discover_skills(spec) {
            let size = fs::metadata(&sk.skill_md_path).map(|m| m.len()).unwrap_or(0);
            copied += 1;
            bytes += size;
            if args.dry_run {
                continue;
            }
            let dst_dir = home.join(spec.slug.as_str()).join(&sk.name);
            fs::create_dir_all(&dst_dir)
                .with_context(|| format!("mkdir {}", dst_dir.display()))?;
            let dst = dst_dir.join("SKILL.md");
            fs::copy(&sk.skill_md_path, &dst)
                .with_context(|| format!("copy {} -> {}", sk.skill_md_path.display(), dst.display()))?;
            upsert_entry(ManifestEntry {
                name: sk.name.clone(),
                source: spec.slug,
                source_path: sk.skill_md_path.to_string_lossy().into_owned(),
                copied_at: chrono_like_now(),
                size_bytes: size,
            })?;
        }
        total += copied;
        total_bytes += bytes;
        writeln!(
            stdout,
            "  {:<14}  {:>4} skill(s)  {:>8} bytes",
            spec.slug.as_str(),
            copied,
            bytes
        )?;
    }
    writeln!(stdout, "done — {total} skill(s), {total_bytes} bytes total.")?;
    if !args.dry_run {
        if let Ok(m) = read_manifest() {
            writeln!(stdout, "manifest: {} entries tracked.", m.entries.len())?;
        }
    }
    Ok(0)
}

fn run_find(args: &FindArgs) -> Result<u8> {
    let rows = collect_rows(&ListArgs {
        path: None,
        source: None,
        json: args.json,
    })?;
    let filtered: Vec<&ListRow> = match args.query.as_deref() {
        Some(q) if !q.is_empty() => {
            let needle = q.to_lowercase();
            rows.iter()
                .filter(|r| {
                    r.name.to_lowercase().contains(&needle)
                        || r.description.to_lowercase().contains(&needle)
                })
                .collect()
        },
        _ => rows.iter().collect(),
    };
    let mut stdout = std::io::stdout().lock();
    if args.json {
        writeln!(stdout, "{}", serde_json::to_string_pretty(&filtered)?)?;
        return Ok(0);
    }
    if filtered.is_empty() {
        writeln!(
            stdout,
            "No skills match {:?}.",
            args.query.as_deref().unwrap_or("")
        )?;
        return Ok(0);
    }
    for r in &filtered {
        writeln!(
            stdout,
            "{}/{}\t{}",
            r.source,
            r.name,
            truncate(&r.description, 80)
        )?;
    }
    writeln!(stdout, "\n{} match(es).", filtered.len())?;
    Ok(0)
}

fn run_init(args: &InitArgs) -> Result<u8> {
    let (dir, skill_name) = match &args.name {
        Some(n) => (PathBuf::from(n), n.clone()),
        None => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let name = cwd
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "my-skill".to_string());
            (PathBuf::from("."), name)
        },
    };
    fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
    let path = dir.join("SKILL.md");
    if path.exists() {
        anyhow::bail!("{} already exists — refusing to overwrite", path.display());
    }
    let template = format!(
        "---\nname: {skill_name}\ndescription: What this skill does and when to use it.\n---\n\n\
         # {skill_name}\n\nInstructions for the agent to follow when this skill is activated.\n\n\
         ## When to Use\n\nDescribe the scenarios where this skill should be used.\n\n\
         ## Steps\n\n1. First, do this.\n2. Then, do that.\n"
    );
    fs::write(&path, template).with_context(|| format!("write {}", path.display()))?;
    writeln!(std::io::stdout(), "created {}", path.display())?;
    Ok(0)
}

fn run_update(args: &UpdateArgs) -> Result<u8> {
    let mut manifest = read_manifest()?;
    if manifest.entries.is_empty() {
        writeln!(std::io::stdout(), "no installed skills to update.")?;
        return Ok(0);
    }
    let home = ensure_home_exists()?;
    let mut updated = 0usize;
    let mut missing = 0usize;
    for entry in &mut manifest.entries {
        if !args.names.is_empty() && !args.names.iter().any(|n| n == &entry.name) {
            continue;
        }
        let src = PathBuf::from(&entry.source_path);
        if !src.is_file() {
            missing += 1;
            writeln!(
                std::io::stderr(),
                "skip {}/{}: source gone ({})",
                entry.source.as_str(),
                entry.name,
                entry.source_path
            )?;
            continue;
        }
        let dst_dir = home.join(entry.source.as_str()).join(&entry.name);
        fs::create_dir_all(&dst_dir).with_context(|| format!("mkdir {}", dst_dir.display()))?;
        let dst = dst_dir.join("SKILL.md");
        fs::copy(&src, &dst)
            .with_context(|| format!("copy {} -> {}", src.display(), dst.display()))?;
        entry.size_bytes = fs::metadata(&src).map(|m| m.len()).unwrap_or(entry.size_bytes);
        entry.copied_at = chrono_like_now();
        updated += 1;
    }
    write_manifest(&manifest)?;
    writeln!(
        std::io::stdout(),
        "updated {updated} skill(s), {missing} skipped (source missing)."
    )?;
    Ok(0)
}

/// Conventional Claude Code plugin base dirs to scan for plugin manifests.
/// Never the hardcoded dev plugin path — only standard locations + an env
/// override (`APHRODY_PLUGIN_BASE`); the manifest resolves the rest.
fn plugin_bases() -> Vec<PathBuf> {
    let mut bases: Vec<PathBuf> = Vec::new();
    if let Ok(b) = std::env::var("APHRODY_PLUGIN_BASE") {
        if !b.is_empty() {
            bases.push(PathBuf::from(b));
        }
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut dir = cwd.clone();
    let mut root = cwd;
    for _ in 0..8 {
        if dir.join(".git").exists() || dir.join("Cargo.toml").exists() {
            root = dir.clone();
            break;
        }
        match dir.parent() {
            Some(p) if p != dir => dir = p.to_path_buf(),
            _ => break,
        }
    }
    bases.push(root.clone());
    bases.push(root.join(".claude").join("plugins"));
    if let Some(h) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        bases.push(PathBuf::from(h).join(".claude").join("plugins"));
    }
    let mut seen = std::collections::HashSet::new();
    bases.retain(|p| seen.insert(p.clone()));
    bases
}

/// `INSTALL_INTERNAL_SKILLS=1` reveals skills marked `metadata.internal: true`.
fn install_internal() -> bool {
    std::env::var("INSTALL_INTERNAL_SKILLS").is_ok_and(|v| v == "1")
}

fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Re-export the formatter via lib? Keep it local to avoid coupling.
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let h = rem / 3_600;
    let m = (rem % 3_600) / 60;
    let s = rem % 60;
    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('\u{2026}');
    out
}
