// Copyright 2026 Yohan Pierre
// SPDX-License-Identifier: Apache-2.0

use n2b_util::{Edit, apply_edits};

#[test]
fn empty_edits_returns_source() {
    assert_eq!(apply_edits("abc", vec![]), "abc");
}

#[test]
fn single_edit_at_start() {
    let edits = vec![Edit {
        index: 0,
        len: 3,
        replacement: "XYZ".into(),
    }];
    assert_eq!(apply_edits("abcdef", edits), "XYZdef");
}

#[test]
fn single_edit_at_end() {
    let edits = vec![Edit {
        index: 3,
        len: 3,
        replacement: "XYZ".into(),
    }];
    assert_eq!(apply_edits("abcdef", edits), "abcXYZ");
}

#[test]
fn multiple_non_overlapping_edits() {
    let edits = vec![
        Edit {
            index: 0,
            len: 1,
            replacement: "A".into(),
        },
        Edit {
            index: 4,
            len: 1,
            replacement: "E".into(),
        },
    ];
    assert_eq!(apply_edits("abcde", edits), "AbcdE");
}

#[test]
fn overlap_keeps_longer_at_same_index() {
    // index égal : la plus longue gagne.
    let edits = vec![
        Edit {
            index: 0,
            len: 3,
            replacement: "SHORT".into(),
        },
        Edit {
            index: 0,
            len: 6,
            replacement: "LONG".into(),
        },
    ];
    assert_eq!(apply_edits("abcdefghi", edits), "LONGghi");
}

#[test]
fn nested_overlap_keeps_outer() {
    // index 0 len 6 enveloppe index 2 len 2 — l'outer gagne.
    let edits = vec![
        Edit {
            index: 0,
            len: 6,
            replacement: "OUTER".into(),
        },
        Edit {
            index: 2,
            len: 2,
            replacement: "in".into(),
        },
    ];
    assert_eq!(apply_edits("abcdefghi", edits), "OUTERghi");
}

#[test]
fn adjacent_edits_both_applied() {
    // pas d'overlap si fin du premier == début du second.
    let edits = vec![
        Edit {
            index: 0,
            len: 2,
            replacement: "AA".into(),
        },
        Edit {
            index: 2,
            len: 2,
            replacement: "BB".into(),
        },
    ];
    assert_eq!(apply_edits("abcd", edits), "AABB");
}
