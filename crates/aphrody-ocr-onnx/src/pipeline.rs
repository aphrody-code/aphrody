// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors

//! End-to-end PP-OCR image execution built from the detector and recognizer.

use image::DynamicImage;

use crate::{DetectedRegion, OnnxOcrError, PpOcrDetector, PpOcrRecognizer, Recognition, Result};

/// One detector region and the text decoded from its image crop.
#[derive(Debug, Clone, PartialEq)]
pub struct RecognisedRegion {
    /// Detector geometry and confidence in original-image coordinates.
    pub region: DetectedRegion,
    /// CTC-decoded text and confidence for the region crop.
    pub recognition: Recognition,
}

/// A paired PP-OCR DB detector and CTC recognizer.
#[derive(Debug)]
pub struct PpOcr {
    detector: PpOcrDetector,
    recognizer: PpOcrRecognizer,
}

impl PpOcr {
    /// Load paired local detector and recognizer artefacts from one catalogue entry.
    ///
    /// # Errors
    ///
    /// Returns an error when either locally installed model cannot create an
    /// ONNX Runtime session with the requested provider policy.
    pub fn load(entry_id: &str, config: &aphrody_infer::SessionConfig) -> Result<Self> {
        Ok(Self {
            detector: PpOcrDetector::load(entry_id, config)?,
            recognizer: PpOcrRecognizer::load(entry_id, config)?,
        })
    }

    /// Decode and recognise one image from disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the file is not a supported image or either ONNX
    /// model rejects its prepared tensor.
    pub fn recognise_path(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Vec<RecognisedRegion>> {
        self.recognise(&image::open(path)?)
    }

    /// Convert a recognised region into the shared portable result contract.
    #[must_use]
    pub fn block(region: RecognisedRegion) -> aphrody_ocr_core::OcrBlock {
        aphrody_ocr_core::OcrBlock {
            text: region.recognition.decoded.text,
            polygon: Some(region.region.points),
            confidence: Some(region.recognition.decoded.confidence),
            role: Some("text".into()),
        }
    }

    /// Report the providers that actually built the detector and recognizer.
    #[must_use]
    pub fn providers(&self) -> [aphrody_models::Accelerator; 2] {
        [self.detector.provider(), self.recognizer.provider()]
    }

    /// Detect and recognise text regions in one image.
    ///
    /// Regions are ordered top-to-bottom then left-to-right. Recognition uses
    /// the smallest axis-aligned crop that contains the detector quadrilateral;
    /// callers handling rotated documents must rectify the quadrilateral before
    /// presenting the crop to this fast-path API.
    ///
    /// # Errors
    ///
    /// Returns an error when detector or recognizer inference fails, or when a
    /// detector region cannot be represented as a non-empty source-image crop.
    pub fn recognise(&mut self, image: &DynamicImage) -> Result<Vec<RecognisedRegion>> {
        let map = self.detector.detect(image)?;
        let mut regions = map.regions();
        regions.sort_unstable_by(|left, right| reading_order(left, right));
        regions
            .into_iter()
            .map(|region| {
                let crop = crop_region(image, &region)?;
                let recognition = self.recognizer.recognise(&crop)?;
                Ok(RecognisedRegion { region, recognition })
            })
            .collect()
    }
}

fn reading_order(left: &DetectedRegion, right: &DetectedRegion) -> std::cmp::Ordering {
    let left_y = left.points.iter().map(|point| point[1]).sum::<f32>() / 4.0;
    let right_y = right.points.iter().map(|point| point[1]).sum::<f32>() / 4.0;
    let left_x = left.points.iter().map(|point| point[0]).sum::<f32>() / 4.0;
    let right_x = right.points.iter().map(|point| point[0]).sum::<f32>() / 4.0;
    left_y.total_cmp(&right_y).then_with(|| left_x.total_cmp(&right_x))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss, clippy::cast_sign_loss)]
fn crop_region(image: &DynamicImage, region: &DetectedRegion) -> Result<DynamicImage> {
    let max_x = image.width().saturating_sub(1) as f32;
    let max_y = image.height().saturating_sub(1) as f32;
    let left = region
        .points
        .iter()
        .map(|point| point[0])
        .fold(f32::INFINITY, f32::min)
        .floor()
        .clamp(0.0, max_x) as u32;
    let top = region
        .points
        .iter()
        .map(|point| point[1])
        .fold(f32::INFINITY, f32::min)
        .floor()
        .clamp(0.0, max_y) as u32;
    let right = region
        .points
        .iter()
        .map(|point| point[0])
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .clamp(0.0, image.width() as f32) as u32;
    let bottom = region
        .points
        .iter()
        .map(|point| point[1])
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .clamp(0.0, image.height() as f32) as u32;
    let width = right.saturating_sub(left);
    let height = bottom.saturating_sub(top);
    if width == 0 || height == 0 {
        return Err(OnnxOcrError::EmptyCrop);
    }
    Ok(image.crop_imm(left, top, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reading_order_uses_vertical_then_horizontal_centres() {
        let at = |x, y| DetectedRegion {
            points: [[x, y], [x + 1.0, y], [x + 1.0, y + 1.0], [x, y + 1.0]],
            confidence: 1.0,
        };
        assert!(reading_order(&at(1.0, 2.0), &at(2.0, 2.0)).is_lt());
        assert!(reading_order(&at(5.0, 1.0), &at(1.0, 2.0)).is_lt());
    }

    #[test]
    #[ignore = "requires locally pulled PP-OCRv5 weights and the repository sample image"]
    fn installed_mobile_pair_runs_on_a_real_image() {
        let config = aphrody_infer::SessionConfig::with_only(aphrody_models::Accelerator::Cpu);
        let mut ocr = PpOcr::load("ppocr-v5-mobile", &config).unwrap();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../google.png");
        let image = image::open(path).unwrap();
        let regions = ocr.recognise(&image).unwrap();
        assert!(!regions.is_empty());
        assert!(regions.iter().any(|region| !region.recognition.decoded.text.is_empty()));
        assert!(regions.iter().all(|region| region.region.confidence.is_finite()
            && region.recognition.decoded.confidence.is_finite()));
    }
}
