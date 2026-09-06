// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors

//! CTC dictionary loading and greedy decoding for PP-OCR exports.

use std::{fs, path::Path};

use serde_yaml::Value;

use crate::{OnnxOcrError, Result};

/// A decoded CTC sequence and the mean score of emitted glyphs.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedText {
    /// UTF-8 text after CTC blank/repetition removal.
    pub text: String,
    /// Mean winning score for emitted glyphs.
    pub confidence: f32,
}

/// PP-OCR CTC class mapping derived from an `inference.yml` export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtcDecoder {
    classes: Vec<String>,
}

impl CtcDecoder {
    /// Load the configured glyphs and reproduce PP-OCRv5 CTC class ordering.
    ///
    /// PP-OCRv5 trains with `use_space_char: true`: blank is prepended and an
    /// ASCII space appended to the serialized glyph list.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or parsed, or does not
    /// contain a non-empty scalar `PostProcess.character_dict` sequence.
    pub fn from_export_config(path: &Path) -> Result<Self> {
        let source = fs::read_to_string(path)
            .map_err(|source| OnnxOcrError::ReadConfig { path: path.to_path_buf(), source })?;
        let root: Value = serde_yaml::from_str(&source)
            .map_err(|source| OnnxOcrError::ParseConfig { path: path.to_path_buf(), source })?;
        let Some(dict) = root
            .get("PostProcess")
            .and_then(Value::as_mapping)
            .and_then(|post| post.get(Value::String("character_dict".into())))
            .and_then(Value::as_sequence)
        else {
            return Err(OnnxOcrError::MissingCharacterDict { path: path.to_path_buf() });
        };
        let glyphs = dict
            .iter()
            .map(Value::as_str)
            .collect::<Option<Vec<_>>>()
            .filter(|glyphs| !glyphs.is_empty())
            .ok_or_else(|| OnnxOcrError::MissingCharacterDict { path: path.to_path_buf() })?;
        let mut classes = Vec::with_capacity(glyphs.len() + 2);
        classes.push("blank".into());
        classes.extend(glyphs.into_iter().map(str::to_owned));
        classes.push(" ".into());
        Ok(Self { classes })
    }

    /// Number of ONNX output classes expected by this decoder.
    #[must_use]
    pub fn class_count(&self) -> usize {
        self.classes.len()
    }

    /// Decode one `[time, class]` probability matrix using greedy CTC.
    #[must_use]
    pub fn decode(&self, rows: &[&[f32]]) -> DecodedText {
        let mut last = None;
        let mut emitted = Vec::new();
        for row in rows {
            let Some((index, score)) =
                row.iter().copied().enumerate().max_by(|a, b| a.1.total_cmp(&b.1))
            else {
                continue;
            };
            if index != 0 && last != Some(index) {
                if let Some(glyph) = self.classes.get(index) {
                    emitted.push((glyph.as_str(), score));
                }
            }
            last = Some(index);
        }
        let confidence = if emitted.is_empty() {
            0.0
        } else {
            // The exported recognizer has at most 320 time steps (input width
            // is fixed at 320), so this conversion is exact in the supported
            // model contract.
            #[allow(clippy::cast_precision_loss)]
            let count = emitted.len() as f32;
            emitted.iter().map(|(_, score)| score).sum::<f32>() / count
        };
        DecodedText { text: emitted.into_iter().map(|(glyph, _)| glyph).collect(), confidence }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greedy_ctc_removes_blanks_and_adjacent_repetitions() {
        let decoder = CtcDecoder { classes: vec!["blank".into(), "A".into(), "B".into()] };
        let rows = [
            &[0.9, 0.1, 0.0][..],
            &[0.1, 0.8, 0.1][..],
            &[0.1, 0.7, 0.2][..],
            &[0.8, 0.1, 0.1][..],
            &[0.1, 0.2, 0.7][..],
        ];
        assert_eq!(decoder.decode(&rows), DecodedText { text: "AB".into(), confidence: 0.75 });
    }
}
