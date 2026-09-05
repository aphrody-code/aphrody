// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors

use pyo3::prelude::*;

#[pyfunction]
fn cosine_similarity(v1: Vec<f32>, v2: Vec<f32>) -> PyResult<f32> {
    if v1.len() != v2.len() || v1.is_empty() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Vectors must be non-empty and of same length",
        ));
    }
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for (a, b) in v1.iter().zip(v2.iter()) {
        dot += a * b;
        norm_a += a * a;
        norm_b += b * b;
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        Ok(0.0)
    } else {
        Ok(dot / (norm_a.sqrt() * norm_b.sqrt()))
    }
}

#[pyfunction]
fn top_k_cosine_similarity(
    query: Vec<f32>,
    embeddings: Vec<Vec<f32>>,
    k: usize,
) -> PyResult<Vec<(usize, f32)>> {
    if query.is_empty() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Query vector must be non-empty",
        ));
    }
    let mut results = Vec::with_capacity(embeddings.len());
    for (idx, emb) in embeddings.iter().enumerate() {
        if emb.len() != query.len() {
            continue;
        }
        let mut dot = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;
        for (a, b) in query.iter().zip(emb.iter()) {
            dot += a * b;
            norm_a += a * a;
            norm_b += b * b;
        }
        let score = if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot / (norm_a.sqrt() * norm_b.sqrt())
        };
        results.push((idx, score));
    }
    results.sort_by(|a, b| {
        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(k);
    Ok(results)
}

#[pyfunction]
#[pyo3(signature = (rankings, k=None))]
fn reciprocal_rank_fusion(
    rankings: Vec<Vec<usize>>,
    k: Option<f32>,
) -> PyResult<Vec<(usize, f32)>> {
    use std::collections::HashMap;
    let k_val = k.unwrap_or(60.0);
    let mut scores = HashMap::new();
    for ranking in rankings {
        for (rank, &item_id) in ranking.iter().enumerate() {
            let score = 1.0 / (k_val + rank as f32);
            *scores.entry(item_id).or_insert(0.0) += score;
        }
    }
    let mut results: Vec<(usize, f32)> = scores.into_iter().collect();
    results.sort_by(|a, b| {
        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(results)
}

#[pymodule]
fn aphrody_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(cosine_similarity, m)?)?;
    m.add_function(wrap_pyfunction!(top_k_cosine_similarity, m)?)?;
    m.add_function(wrap_pyfunction!(reciprocal_rank_fusion, m)?)?;
    Ok(())
}
