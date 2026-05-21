// SPDX-License-Identifier: Apache-2.0
//! Multi-chunk merge.
//!
//! When the parser yields multiple [`Artifact`] events with the **same id**
//! (e.g. a streamed update or a continuation chunk), the merge layer is the
//! single authority for combining them.
//!
//! Rules:
//!
//! * Same id + same `type_` ⇒ concatenate `content`, last `title` wins.
//! * Same id + different `type_` ⇒ [`SidecarError::Merge`] — the artifact
//!   format must be stable across chunks.
//! * Duplicate-byte-for-byte content ⇒ deduplicated (no double append).
//! * Insertion order is preserved across distinct ids (so the preview UI
//!   sees artifacts in the order the LLM emitted them).

use std::collections::HashMap;

use crate::sidecar::error::{SidecarError, SidecarResult};
use crate::sidecar::parser::Artifact;

/// Merge a list of artifact events into the canonical (deduplicated,
/// concatenated) list.
///
/// # Errors
///
/// [`SidecarError::Merge`] if two chunks share an id but disagree on type.
pub fn merge_chunks(events: Vec<Artifact>) -> SidecarResult<Vec<Artifact>> {
    let mut order: Vec<String> = Vec::new();
    let mut by_id: HashMap<String, Artifact> = HashMap::new();

    for ev in events {
        if let Some(existing) = by_id.get_mut(&ev.id) {
            merge_into(existing, &ev)?;
        } else {
            order.push(ev.id.clone());
            by_id.insert(ev.id.clone(), ev);
        }
    }

    Ok(order.into_iter().filter_map(|id| by_id.remove(&id)).collect())
}

/// In-place merge of `incoming` into `target`. Same rules as
/// [`merge_chunks`].
///
/// # Errors
///
/// [`SidecarError::Merge`] if types disagree.
pub fn merge_into(target: &mut Artifact, incoming: &Artifact) -> SidecarResult<()> {
    if target.id != incoming.id {
        return Err(SidecarError::Merge(format!(
            "merge target id {} != incoming {}",
            target.id, incoming.id
        )));
    }
    if target.type_ != incoming.type_ {
        return Err(SidecarError::Merge(format!(
            "artifact {} type changed across chunks: {} -> {}",
            target.id, target.type_, incoming.type_
        )));
    }
    if !incoming.title.is_empty() {
        target.title = incoming.title.clone();
    }
    // Dedup: skip the append if the incoming content is already a suffix.
    if target.content == incoming.content {
        return Ok(());
    }
    if target.content.ends_with(&incoming.content) {
        return Ok(());
    }
    target.content.push_str(&incoming.content);
    target.mime = incoming.mime.clone();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn art(id: &str, type_: &str, content: &str) -> Artifact {
        Artifact {
            id: id.to_string(),
            type_: type_.to_string(),
            title: id.to_string(),
            content: content.to_string(),
            mime: type_.to_string(),
        }
    }

    #[test]
    fn concatenates_same_id_same_type() {
        let out = merge_chunks(vec![
            art("doc", "text/markdown", "## Part 1\n"),
            art("doc", "text/markdown", "## Part 2\n"),
        ])
        .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].content, "## Part 1\n## Part 2\n");
    }

    #[test]
    fn dedup_identical_chunk() {
        let out = merge_chunks(vec![
            art("d", "text/markdown", "hi"),
            art("d", "text/markdown", "hi"),
        ])
        .unwrap();
        assert_eq!(out[0].content, "hi");
    }

    #[test]
    fn preserves_insertion_order_across_ids() {
        let out = merge_chunks(vec![
            art("b", "text/markdown", "B"),
            art("a", "text/markdown", "A"),
            art("c", "text/markdown", "C"),
            art("a", "text/markdown", "A2"),
        ])
        .unwrap();
        let ids: Vec<&str> = out.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["b", "a", "c"]);
        // "a" got the second chunk appended.
        let a = out.iter().find(|x| x.id == "a").unwrap();
        assert_eq!(a.content, "AA2");
    }

    #[test]
    fn rejects_type_drift() {
        let err = merge_chunks(vec![
            art("d", "text/markdown", "hi"),
            art("d", "text/html", "<b>hi</b>"),
        ])
        .unwrap_err();
        assert!(matches!(err, SidecarError::Merge(_)));
    }
}
