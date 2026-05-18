// SPDX-License-Identifier: Apache-2.0
//
// a2a-duel-loop — one tick of the A2A duel between the aphrody Claude and the
// winclean Claude. Reads the peer's last envelope from
// `.coord/inbox-from-<peer>.jsonl`, composes a reply, appends it to the
// correct inbox JSONL, mirrors via the HTTP listener when reachable, and
// bumps the matching heartbeat file.
//
// Pure-Rust port of `.claude/plugins/aphrody/skills/a2a-duel-loop/scripts/duel-cycle.ts`.

#![doc = "a2a-duel-loop binary — single tick driver for the file-based A2A duel."]

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use clap::{Parser, ValueEnum};
use getrandom::getrandom;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lowercase")]
enum Side {
    Aphrody,
    Winclean,
}

impl Side {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Aphrody => "aphrody",
            Self::Winclean => "winclean",
        }
    }

    const fn peer(self) -> Self {
        match self {
            Self::Aphrody => Self::Winclean,
            Self::Winclean => Self::Aphrody,
        }
    }

    const fn from_id(self) -> &'static str {
        match self {
            Self::Aphrody => "aphrody@aphrody-code/aphrody",
            Self::Winclean => "winclean@aphrody-code/winclean",
        }
    }

    const fn to_id(self) -> &'static str {
        self.peer().from_id()
    }

    const fn id_prefix(self) -> &'static str {
        match self {
            Self::Aphrody => "apx-duel",
            Self::Winclean => "wc-duel",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[value(rename_all = "lowercase")]
enum EnvType {
    Ping,
    Ask,
    Fact,
    Ack,
}

impl EnvType {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Ping => "ping",
            Self::Ask => "ask",
            Self::Fact => "fact",
            Self::Ack => "ack",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Envelope {
    id: String,
    ts: String,
    from: String,
    to: String,
    #[serde(rename = "type")]
    env_type: EnvType,
    re: Option<String>,
    subject: String,
    body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel_hint: Option<Vec<String>>,
}

#[derive(Debug, Parser)]
#[command(
    name = "a2a-duel-loop",
    about = "Single tick of the A2A duel between aphrody and winclean Claudes.",
    long_about = "Reads the peer's last envelope from inbox-from-<peer>.jsonl, composes a reply, \
                  appends it to inbox-from-<side>.jsonl, optionally POSTs to the local listener, \
                  and bumps heartbeat-<side>.txt."
)]
struct Cli {
    /// Tick number (integer >= 1).
    #[arg(long)]
    iteration: u64,

    /// Which Claude is speaking this tick.
    #[arg(long, value_enum, default_value_t = Side::Aphrody)]
    side: Side,

    /// Override coord root (default: C:/winclean/.coord on Windows, $APHRODY_COORD_DIR otherwise).
    #[arg(long, env = "APHRODY_COORD_DIR")]
    coord_dir: Option<PathBuf>,

    /// Override subject line.
    #[arg(long)]
    subject: Option<String>,

    /// Override body text.
    #[arg(long)]
    body: Option<String>,

    /// Envelope type.
    #[arg(long = "type", value_enum, default_value_t = EnvType::Ping)]
    env_type: EnvType,

    /// Envelope `re` field for ack.
    #[arg(long)]
    re: Option<String>,

    /// Skip POST /msg to the listener.
    #[arg(long)]
    no_http: bool,

    /// Print the envelope, do not write.
    #[arg(long)]
    dry_run: bool,
}

fn default_coord_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        PathBuf::from("C:/winclean/.coord")
    } else {
        // XDG-ish fallback on non-Windows hosts. The duel is Windows-anchored
        // (winclean lives on Windows), so non-Windows runs require an explicit
        // override via --coord-dir or APHRODY_COORD_DIR.
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join(".local/share/winclean/.coord"))
            .unwrap_or_else(|| PathBuf::from("./.coord"))
    }
}

fn now_utc_iso() -> String {
    // Match the TS implementation which strips milliseconds: `YYYY-MM-DDTHH:MM:SSZ`.
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn rand8_hex() -> String {
    let mut buf = [0_u8; 4];
    if getrandom(&mut buf).is_err() {
        // Fallback to timestamp-derived bytes if the OS RNG is unavailable.
        let nanos = Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
        buf.copy_from_slice(&nanos.to_le_bytes()[..4]);
    }
    hex::encode(buf)
}

fn envelope_id(side: Side, iteration: u64) -> String {
    format!("{}-{}-{}", side.id_prefix(), iteration, rand8_hex())
}

fn read_last_envelope(path: &Path) -> Option<Envelope> {
    let mut file = File::open(path).ok()?;
    let mut content = String::new();
    file.read_to_string(&mut content).ok()?;
    let last_line = content
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())?;
    serde_json::from_str::<Envelope>(last_line).ok()
}

fn compose(cli: &Cli, peer_last: Option<&Envelope>) -> Envelope {
    let default_subject = format!("duel tick {}", cli.iteration);
    let mut default_body = format!("Tick {} from {}.", cli.iteration, cli.side.as_str());
    if let Some(env) = peer_last {
        default_body.push_str(&format!(
            " Peer's last envelope: id={} type={} subject=\"{}\".",
            env.id,
            env.env_type.as_str(),
            env.subject,
        ));
    } else {
        default_body.push_str(" No prior peer envelope visible.");
    }
    default_body.push_str(&format!(
        " heartbeat-{}.txt bumped to {}.",
        cli.side.as_str(),
        now_utc_iso(),
    ));

    Envelope {
        id: envelope_id(cli.side, cli.iteration),
        ts: now_utc_iso(),
        from: cli.side.from_id().to_owned(),
        to: cli.side.to_id().to_owned(),
        env_type: cli.env_type,
        re: cli.re.clone(),
        subject: cli.subject.clone().unwrap_or(default_subject),
        body: cli.body.clone().unwrap_or(default_body),
        channel_hint: Some(vec!["file_jsonl".to_owned(), "http_jsonrpc".to_owned()]),
    }
}

#[cfg(not(target_family = "wasm"))]
async fn try_post_msg(env: &Envelope) -> bool {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(2000))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    client
        .post("http://localhost:8788/msg")
        .header("Content-Type", "application/json")
        .json(env)
        .send()
        .await
        .is_ok_and(|r| r.status().is_success())
}

#[cfg(target_family = "wasm")]
async fn try_post_msg(_env: &Envelope) -> bool {
    false
}

fn run() -> ExitCode {
    let cli = Cli::parse();

    if cli.iteration < 1 {
        eprintln!("error: --iteration N (integer >= 1) is required");
        return ExitCode::from(1);
    }

    let coord_dir = cli.coord_dir.clone().unwrap_or_else(default_coord_dir);
    if !coord_dir.exists() {
        eprintln!("error: coord dir not found: {}", coord_dir.display());
        eprintln!("hint: pass --coord-dir <path> or create the directory first.");
        return ExitCode::from(2);
    }

    let peer = cli.side.peer();
    let peer_inbox = coord_dir.join(format!("inbox-from-{}.jsonl", peer.as_str()));
    let my_inbox = coord_dir.join(format!("inbox-from-{}.jsonl", cli.side.as_str()));
    let heartbeat = coord_dir.join(format!("heartbeat-{}.txt", cli.side.as_str()));

    let peer_last = read_last_envelope(&peer_inbox);
    let env = compose(&cli, peer_last.as_ref());

    match serde_json::to_string_pretty(&env) {
        Ok(s) => println!("{s}"),
        Err(e) => {
            eprintln!("error: failed to serialize envelope: {e}");
            return ExitCode::from(3);
        },
    }

    if cli.dry_run {
        println!("--dry-run: not writing");
        return ExitCode::SUCCESS;
    }

    let line = match serde_json::to_string(&env) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("write failed: serialize: {e}");
            return ExitCode::from(3);
        },
    };

    if let Err(e) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&my_inbox)
        .and_then(|mut f| {
            f.write_all(line.as_bytes())?;
            f.write_all(b"\n")
        })
    {
        eprintln!("write failed: {e}");
        return ExitCode::from(3);
    }

    if let Err(e) = std::fs::write(&heartbeat, format!("{}\n", now_utc_iso())) {
        eprintln!("write failed: {e}");
        return ExitCode::from(3);
    }

    if !cli.no_http {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("error: failed to build tokio runtime: {e}");
                return ExitCode::from(3);
            },
        };
        let ok = rt.block_on(try_post_msg(&env));
        println!(
            "POST /msg: {}",
            if ok { "200 OK" } else { "skipped (listener unreachable)" }
        );
    }

    println!("appended to {}", my_inbox.display());
    println!("bumped {}", heartbeat.display());
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    run()
}
