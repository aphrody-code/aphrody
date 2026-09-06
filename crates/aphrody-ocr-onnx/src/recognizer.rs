// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors

//! PP-OCRv5 crop recognition through the shared ONNX Runtime session loader.

use image::{DynamicImage, imageops::FilterType};
use ndarray::{Array4, Axis};
use ort::value::TensorRef;

use crate::{CtcDecoder, DecodedText, OnnxOcrError, Result};

/// The fixed recognition height in the PP-OCRv5 mobile export.
const REC_HEIGHT: u32 = 48;
/// The maximum recognition width in the PP-OCRv5 mobile export.
const REC_WIDTH: u32 = 320;

/// Text extracted from one already-localised text crop.
#[derive(Debug, Clone, PartialEq)]
pub struct Recognition {
    /// Decoded CTC text and confidence.
    pub decoded: DecodedText,
    /// Width used before right-padding, for diagnostic reproducibility.
    pub content_width: u32,
}

/// A loaded PP-OCR recognizer and its export-derived CTC mapping.
#[derive(Debug)]
pub struct PpOcrRecognizer {
    model: aphrody_infer::LoadedModel,
    decoder: CtcDecoder,
}

impl PpOcrRecognizer {
    /// Load a recognizer and its matching `recognizer-config` from the model
    /// catalogue.
    ///
    /// Both artefacts must have been pulled already; no download occurs here.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested catalogue roles are missing or not
    /// installed, the export configuration is invalid, or ONNX Runtime cannot
    /// construct a session with the supplied execution-provider policy.
    pub fn load(entry_id: &str, config: &aphrody_infer::SessionConfig) -> Result<Self> {
        use aphrody_models::{Catalog, ModelStore};

        let entry = Catalog::builtin().get(entry_id)?;
        let config_file =
            entry.file("recognizer-config").ok_or_else(|| OnnxOcrError::MissingCatalogRole {
                entry: entry_id.to_owned(),
                role: "recognizer-config".into(),
            })?;
        let store = ModelStore::open()?;
        let config_path = store
            .get(&config_file.reference)?
            .ok_or_else(|| {
                aphrody_models::ModelError::NotInstalled(config_file.reference.to_string())
            })?
            .path;
        let decoder = CtcDecoder::from_export_config(&config_path)?;
        let model = aphrody_infer::load_catalog_role(entry_id, "recognizer", config)?;
        Ok(Self { model, decoder })
    }

    /// Recognise an already-localised text crop.
    ///
    /// The crop is resized while preserving aspect ratio, converted RGB to the
    /// model's BGR channel order, normalised by `(value / 255 - .5) / .5`, and
    /// right-padded with zeroes in normalized tensor space.
    ///
    /// # Errors
    ///
    /// Returns an error when ONNX Runtime rejects the prepared tensor, the
    /// model output is not `[1, time, class]`, or the output class count does
    /// not exactly match the downloaded CTC dictionary.
    pub fn recognise(&mut self, crop: &DynamicImage) -> Result<Recognition> {
        let (input, content_width) = preprocess(crop);
        let input_name = self
            .model
            .session
            .inputs()
            .first()
            .ok_or_else(|| OnnxOcrError::MissingTensorSlot { kind: "input", count: 0 })?
            .name()
            .to_owned();
        let output = self
            .model
            .session
            .run(ort::inputs![input_name => TensorRef::from_array_view(&input)?])?;
        let output_count = output.len();
        if output_count == 0 {
            return Err(OnnxOcrError::MissingTensorSlot { kind: "output", count: output_count });
        }
        let logits = output[0].try_extract_array::<f32>()?;
        let shape = logits.shape();
        if shape.len() != 3 || shape[0] != 1 {
            return Err(OnnxOcrError::UnexpectedOutputShape { shape: shape.to_vec() });
        }
        if shape[2] != self.decoder.class_count() {
            return Err(OnnxOcrError::ClassCountMismatch {
                model: shape[2],
                decoder: self.decoder.class_count(),
            });
        }
        let sequence = logits.index_axis(Axis(0), 0);
        let data = sequence.as_slice().ok_or(OnnxOcrError::NonContiguousOutput)?;
        let rows = data.chunks_exact(shape[2]).collect::<Vec<_>>();
        Ok(Recognition { decoded: self.decoder.decode(&rows), content_width })
    }

    /// Report the execution provider that built the recognizer session.
    #[must_use]
    pub fn provider(&self) -> aphrody_models::Accelerator {
        self.model.provider
    }
}

/// Prepare a PP-OCRv5 mobile recognition tensor and its non-padded width.
fn preprocess(crop: &DynamicImage) -> (Array4<f32>, u32) {
    let width = crop.width().max(1);
    let height = crop.height().max(1);
    let scaled = ((u64::from(width) * u64::from(REC_HEIGHT) + u64::from(height) - 1)
        / u64::from(height))
    .clamp(1, u64::from(REC_WIDTH));
    let content_width = u32::try_from(scaled).expect("PP-OCR configured width fits u32");
    let pixels = crop.resize_exact(content_width, REC_HEIGHT, FilterType::Lanczos3).to_rgb8();
    let mut tensor = Array4::zeros((1, 3, REC_HEIGHT as usize, REC_WIDTH as usize));
    for y in 0..REC_HEIGHT {
        for x in 0..content_width {
            let [r, g, b] = pixels.get_pixel(x, y).0;
            tensor[[0, 0, y as usize, x as usize]] = f32::from(b) / 127.5 - 1.0;
            tensor[[0, 1, y as usize, x as usize]] = f32::from(g) / 127.5 - 1.0;
            tensor[[0, 2, y as usize, x as usize]] = f32::from(r) / 127.5 - 1.0;
        }
    }
    (tensor, content_width)
}

#[cfg(test)]
mod tests {
    use image::{Rgb, RgbImage};

    use super::*;

    #[test]
    fn preprocessing_uses_bgr_normalisation_and_zero_padding() {
        let crop = DynamicImage::ImageRgb8(RgbImage::from_pixel(2, 1, Rgb([255, 127, 0])));
        let (tensor, width) = preprocess(&crop);
        assert_eq!(width, 96);
        assert_eq!(tensor.shape(), &[1, 3, 48, 320]);
        assert!((tensor[[0, 0, 0, 0]] + 1.0).abs() < f32::EPSILON);
        assert!((tensor[[0, 1, 0, 0]] + 0.003_921_57).abs() < 0.000_001);
        assert!((tensor[[0, 2, 0, 0]] - 1.0).abs() < f32::EPSILON);
        assert!(tensor[[0, 0, 0, 96]].abs() < f32::EPSILON);
    }

    #[test]
    #[ignore = "requires locally pulled PP-OCRv5 weights and ONNX Runtime"]
    fn installed_mobile_model_accepts_a_real_preprocessed_crop() {
        let config = aphrody_infer::SessionConfig::with_only(aphrody_models::Accelerator::Cpu);
        let mut recognizer = PpOcrRecognizer::load("ppocr-v5-mobile", &config).unwrap();
        let crop = DynamicImage::ImageRgb8(RgbImage::from_pixel(240, 48, Rgb([255, 255, 255])));
        let result = recognizer.recognise(&crop).unwrap();
        assert!(result.decoded.confidence.is_finite());
        assert!((1..=320).contains(&result.content_width));
    }
}
