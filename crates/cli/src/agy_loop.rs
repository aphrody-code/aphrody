// SPDX-License-Identifier: Apache-2.0
//! `aphrody agy-loop …` — pousse le CLI Antigravity (`agy`) dans une boucle de
//! codage autonome via son hook de cycle de vie `AfterAgent`.
//!
//! `agy` n'a pas de mode natif « continue jusqu'à terminé ». Cette sous-commande
//! est le binaire de hook câblé par le plugin aphrody (`hooks/hooks.json`) : à
//! chaque évènement `AfterAgent`, elle lit le JSON du hook sur stdin et, tant
//! qu'une boucle est active et inachevée, émet `{"decision":"deny","reason":…}`
//! que `agy` interprète comme « rejette cet arrêt, génère un nouveau tour » —
//! forçant l'agent à continuer de coder. La boucle se termine quand l'agent émet
//! le jeton sentinelle [`DONE_SENTINEL`], qu'un arrêt est demandé, ou que le
//! plafond d'itérations est atteint (garde anti-emballement).
//!
//! Pur Rust, lit stdin / écrit un fichier d'état : identique sur Linux (cible
//! #1), Windows et macOS — aucune dépendance shell/interpréteur.

use std::{
    io::Read as _,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::json;

/// Jeton que l'agent doit émettre, seul sur sa ligne, pour signaler que
/// l'objectif est réellement atteint, vérifié et committé. Tant qu'il n'apparaît
/// pas dans la réponse, la boucle relance l'agent.
const DONE_SENTINEL: &str = "APHRODY_LOOP_DONE";

/// Chemin du fichier d'état de la boucle, relatif au workspace courant.
const STATE_REL: &str = ".agents/aphrody-loop.json";

/// Marqueur d'arrêt explicite (créé par `agy-loop stop`), relatif au workspace.
const STOP_REL: &str = ".agents/aphrody-loop.stop";

/// Plafond d'itérations par défaut (garde anti-emballement / anti-coût).
const DEFAULT_MAX_ITERATIONS: u32 = 50;

/// Actions de la sous-commande `agy-loop`.
#[derive(clap::Subcommand, Debug, Clone)]
pub(crate) enum AgyLoopAction {
    /// Arme la boucle pour le workspace courant avec un objectif.
    ///
    /// Exemple : aphrody agy-loop start --goal "implémente l'auth OAuth, tests verts" --max 40
    Start {
        /// Objectif de codage réinjecté à l'agent à chaque tour.
        #[arg(long)]
        goal: String,
        /// Plafond d'itérations avant arrêt forcé (garde anti-emballement).
        #[arg(long, default_value_t = DEFAULT_MAX_ITERATIONS)]
        max: u32,
    },
    /// Désarme la boucle (efface l'état) pour le workspace courant.
    Stop,
    /// Affiche l'état courant de la boucle en JSON.
    Status,
    /// Driver du hook `AfterAgent` : lit le JSON du hook sur stdin, émet une
    /// décision sur stdout. Câblé par le plugin agy — pas pour usage manuel.
    Hook,
}

/// État persistant d'une boucle de codage active.
#[derive(Serialize, Deserialize, Debug, Clone)]
struct LoopState {
    /// Objectif réinjecté à chaque tour.
    goal: String,
    /// Nombre de relances déjà forcées.
    iteration: u32,
    /// Plafond d'itérations.
    max_iterations: u32,
    /// Horodatage ISO 8601 d'armement (informatif).
    started_at: String,
}

/// Résout le workspace : `cwd` fourni par le hook, sinon répertoire courant.
fn workspace_from(cwd: Option<&str>) -> PathBuf {
    cwd.filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn state_path(ws: &Path) -> PathBuf {
    ws.join(STATE_REL)
}

fn stop_path(ws: &Path) -> PathBuf {
    ws.join(STOP_REL)
}

fn load_state(ws: &Path) -> Option<LoopState> {
    let raw = std::fs::read_to_string(state_path(ws)).ok()?;
    serde_json::from_str(&raw).ok()
}

fn save_state(ws: &Path, st: &LoopState) -> miette::Result<()> {
    let path = state_path(ws);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| miette::miette!("create .agents dir: {e}"))?;
    }
    let body =
        serde_json::to_string_pretty(st).map_err(|e| miette::miette!("encode loop state: {e}"))?;
    std::fs::write(&path, body).map_err(|e| miette::miette!("write loop state: {e}"))
}

fn clear_state(ws: &Path) {
    let _ = std::fs::remove_file(state_path(ws));
    let _ = std::fs::remove_file(stop_path(ws));
}

/// Directive de relance réinjectée à l'agent quand la boucle force un tour.
fn directive(st: &LoopState) -> String {
    format!(
        "Tu n'as PAS terminé (itération {iter}/{max} de la boucle aphrody). OBJECTIF : \
         {goal}\n\nContinue en autonomie totale, sans poser de question : implémente la prochaine \
         étape concrète avec du vrai code (zéro stub, zéro TODO), lance le build et les tests, \
         corrige les échecs. Si tout est déjà implémenté, durcis les tests et la doc. Quand — et \
         seulement quand — l'objectif est entièrement implémenté, vérifié vert (build + tests) et \
         committé, écris le jeton exact `{done}` seul sur sa dernière ligne. Sinon, reprends le \
         travail maintenant.",
        iter = st.iteration,
        max = st.max_iterations,
        goal = st.goal,
        done = DONE_SENTINEL,
    )
}

/// Émet une réponse de hook JSON sur stdout (un objet par ligne).
fn emit(value: &serde_json::Value) {
    println!("{value}");
}

/// Pilote du hook `AfterAgent`.
fn run_hook() -> miette::Result<()> {
    let mut buf = String::new();
    // Tolérant : stdin vide / illisible ⇒ no-op (ne jamais casser agy).
    let _ = std::io::stdin().read_to_string(&mut buf);
    let v: serde_json::Value = serde_json::from_str(&buf).unwrap_or(serde_json::Value::Null);

    let ws = workspace_from(v.get("cwd").and_then(serde_json::Value::as_str));

    // Boucle inactive ⇒ no-op : on n'interfère jamais avec un arrêt normal.
    let Some(mut st) = load_state(&ws) else {
        emit(&json!({}));
        return Ok(());
    };

    let response = v.get("prompt_response").and_then(serde_json::Value::as_str).unwrap_or("");
    let stop_requested = stop_path(&ws).exists();

    // Terminaison : sentinelle émise ou arrêt explicite demandé.
    if response.contains(DONE_SENTINEL) || stop_requested {
        let iterations = st.iteration;
        clear_state(&ws);
        emit(&json!({
            "continue": true,
            "suppressOutput": true,
            "systemMessage": format!(
                "aphrody agy-loop : objectif atteint après {iterations} itération(s)."
            ),
        }));
        return Ok(());
    }

    // Garde anti-emballement : plafond atteint ⇒ on rend la main.
    if st.iteration >= st.max_iterations {
        let max = st.max_iterations;
        clear_state(&ws);
        emit(&json!({
            "continue": true,
            "systemMessage": format!(
                "aphrody agy-loop : plafond d'itérations ({max}) atteint — arrêt pour \
                 éviter l'emballement. Réarme avec `aphrody agy-loop start`."
            ),
        }));
        return Ok(());
    }

    // Sinon : force un nouveau tour de codage.
    st.iteration += 1;
    save_state(&ws, &st)?;
    emit(&json!({ "decision": "deny", "reason": directive(&st) }));
    Ok(())
}

/// Exécute une action `agy-loop`.
///
/// # Errors
///
/// Renvoie un rapport `miette` sur échec d'IO (lecture/écriture de l'état).
pub(crate) fn run(action: AgyLoopAction) -> miette::Result<()> {
    let ws = workspace_from(None);
    match action {
        AgyLoopAction::Start { goal, max } => {
            let st = LoopState {
                goal,
                iteration: 0,
                max_iterations: max.max(1),
                started_at: now_iso8601(),
            };
            save_state(&ws, &st)?;
            // Un éventuel marqueur d'arrêt résiduel doit être levé au (ré)armement.
            let _ = std::fs::remove_file(stop_path(&ws));
            println!(
                "aphrody agy-loop armé (max {} itérations) dans {}\nobjectif : {}",
                st.max_iterations,
                state_path(&ws).display(),
                st.goal,
            );
            Ok(())
        },
        AgyLoopAction::Stop => {
            // Pose le marqueur d'arrêt (capté par le prochain hook) ET efface
            // l'état immédiatement pour les invocations hors hook.
            if let Some(parent) = stop_path(&ws).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(stop_path(&ws), b"stop\n");
            clear_state(&ws);
            println!("aphrody agy-loop désarmé pour {}", ws.display());
            Ok(())
        },
        AgyLoopAction::Status => {
            match load_state(&ws) {
                Some(st) => {
                    let rendered = serde_json::to_string_pretty(&st)
                        .map_err(|e| miette::miette!("encode loop state: {e}"))?;
                    println!("{rendered}");
                },
                None => println!("(boucle inactive dans {})", ws.display()),
            }
            Ok(())
        },
        AgyLoopAction::Hook => run_hook(),
    }
}

/// Horodatage ISO 8601 UTC sans dépendance externe lourde (via `std::time`).
fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    // Conversion epoch → date civile (algorithme de Howard Hinnant, jours civils).
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    format!("{year:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_ws() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("aphrody-agy-loop-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn start_then_status_roundtrip() {
        let ws = tmp_ws();
        let st = LoopState {
            goal: "ship feature X".to_owned(),
            iteration: 0,
            max_iterations: 10,
            started_at: now_iso8601(),
        };
        save_state(&ws, &st).unwrap();
        let back = load_state(&ws).expect("state persisted");
        assert_eq!(back.goal, "ship feature X");
        assert_eq!(back.max_iterations, 10);
        assert_eq!(back.iteration, 0);
        clear_state(&ws);
        assert!(load_state(&ws).is_none());
    }

    #[test]
    fn directive_embeds_goal_and_sentinel() {
        let st = LoopState {
            goal: "fix the parser".to_owned(),
            iteration: 3,
            max_iterations: 50,
            started_at: now_iso8601(),
        };
        let d = directive(&st);
        assert!(d.contains("fix the parser"));
        assert!(d.contains(DONE_SENTINEL));
        assert!(d.contains("3/50"));
    }

    #[test]
    fn iso8601_shape() {
        let s = now_iso8601();
        // YYYY-MM-DDTHH:MM:SSZ ⇒ 20 caractères.
        assert_eq!(s.len(), 20, "{s}");
        assert!(s.ends_with('Z'));
        assert_eq!(&s[4..5], "-");
        assert_eq!(&s[10..11], "T");
    }

    #[test]
    fn workspace_falls_back_to_cwd() {
        let ws = workspace_from(Some(""));
        assert!(ws.is_absolute() || ws == Path::new("."));
        let ws2 = workspace_from(Some("/tmp/x"));
        assert_eq!(ws2, Path::new("/tmp/x"));
    }
}
