// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors

//! PP-OCRv5 DB detector execution and exact exported input normalisation.

use image::{DynamicImage, imageops::FilterType};
use ndarray::Array4;
use ort::value::TensorRef;

use crate::{OnnxOcrError, Result};

const RESIZE_LONG: u32 = 960;
const ALIGNMENT: u32 = 128;
const BITMAP_THRESHOLD: f32 = 0.3;
const BOX_THRESHOLD: f32 = 0.6;
const UNCLIP_RATIO: f32 = 1.5;
const MIN_SIDE: f32 = 3.0;
const MAX_CANDIDATES: usize = 1_000;

/// Raw DB detector score map together with its reversible image transform.
#[derive(Debug, Clone, PartialEq)]
pub struct DetectionMap {
    /// Detector score map in row-major order, one score per pixel.
    pub scores: Vec<f32>,
    /// Width of the score map.
    pub width: u32,
    /// Height of the score map.
    pub height: u32,
    /// Detector input width after resize and 32-pixel alignment.
    pub input_width: u32,
    /// Detector input height after resize and 32-pixel alignment.
    pub input_height: u32,
    /// Original input image width.
    pub original_width: u32,
    /// Original input image height.
    pub original_height: u32,
}

/// One DB detector candidate in the original image coordinate system.
#[derive(Debug, Clone, PartialEq)]
pub struct DetectedRegion {
    /// Clockwise quadrilateral corners, starting from the top-left corner.
    pub points: [[f32; 2]; 4],
    /// Mean detector probability over the candidate quadrilateral.
    pub confidence: f32,
}

impl DetectionMap {
    /// Convert the DB probability map into scored, expanded text quadrilaterals.
    ///
    /// This follows the exported `DBPostProcess` policy: threshold at 0.3,
    /// reject regions below 0.6, unclip at 1.5, reject sides shorter than five
    /// score-map pixels, then map coordinates to the original image.
    #[must_use]
    pub fn regions(&self) -> Vec<DetectedRegion> {
        if self.width == 0
            || self.height == 0
            || self.scores.len() != self.width as usize * self.height as usize
        {
            return Vec::new();
        }
        let components = connected_components(self);
        let mut regions = Vec::new();
        for component in components.into_iter().take(MAX_CANDIDATES) {
            let Some(rect) = minimum_area_rectangle(&component) else {
                continue;
            };
            if rect.short_side() < MIN_SIDE {
                continue;
            }
            let confidence = mean_score(self, &rect);
            if confidence < BOX_THRESHOLD {
                continue;
            }
            let expanded = rect.unclip(UNCLIP_RATIO);
            if expanded.short_side() < MIN_SIDE + 2.0 {
                continue;
            }
            regions.push(DetectedRegion { points: expanded.to_original(self), confidence });
        }
        regions
    }
}

/// A loaded PP-OCR DB detector.
#[derive(Debug)]
pub struct PpOcrDetector {
    model: aphrody_infer::LoadedModel,
}

impl PpOcrDetector {
    /// Load the detector role from a previously pulled catalogue entry.
    ///
    /// # Errors
    ///
    /// Returns an error when the entry is not installed or ONNX Runtime cannot
    /// build the detector session under the supplied provider policy.
    pub fn load(entry_id: &str, config: &aphrody_infer::SessionConfig) -> Result<Self> {
        Ok(Self { model: aphrody_infer::load_catalog_role(entry_id, "detector", config)? })
    }

    /// Run the DB detector and return its raw probability map.
    ///
    /// Region contouring, unclip and reading-order policy intentionally live
    /// above this primitive so every accepted region remains auditable.
    ///
    /// # Errors
    ///
    /// Returns an error when ONNX Runtime rejects the tensor or the model does
    /// not return a single-batch, one-channel score map.
    pub fn detect(&mut self, image: &DynamicImage) -> Result<DetectionMap> {
        let original_width = image.width();
        let original_height = image.height();
        let (input, input_width, input_height) = preprocess(image);
        let input_name = self
            .model
            .session
            .inputs()
            .first()
            .ok_or_else(|| OnnxOcrError::MissingTensorSlot { kind: "input", count: 0 })?
            .name()
            .to_owned();
        let outputs = self
            .model
            .session
            .run(ort::inputs![input_name => TensorRef::from_array_view(&input)?])?;
        if outputs.len() == 0 {
            return Err(OnnxOcrError::MissingTensorSlot { kind: "output", count: 0 });
        }
        let output = outputs[0].try_extract_array::<f32>()?;
        let shape = output.shape();
        if shape.len() != 4 || shape[0] != 1 || shape[1] != 1 {
            return Err(OnnxOcrError::UnexpectedOutputShape { shape: shape.to_vec() });
        }
        let width = u32::try_from(shape[3])
            .map_err(|_| OnnxOcrError::UnexpectedOutputShape { shape: shape.to_vec() })?;
        let height = u32::try_from(shape[2])
            .map_err(|_| OnnxOcrError::UnexpectedOutputShape { shape: shape.to_vec() })?;
        let scores = output.iter().copied().collect();
        Ok(DetectionMap {
            scores,
            width,
            height,
            input_width,
            input_height,
            original_width,
            original_height,
        })
    }

    /// Report the execution provider that built the detector session.
    #[must_use]
    pub fn provider(&self) -> aphrody_models::Accelerator {
        self.model.provider
    }
}

/// Reproduce the detector export's BGR/ImageNet preprocessing.
fn preprocess(image: &DynamicImage) -> (Array4<f32>, u32, u32) {
    let source_width = image.width().max(1);
    let source_height = image.height().max(1);
    let long_side = source_width.max(source_height);
    let input_width = align(
        u32::try_from(u64::from(source_width) * u64::from(RESIZE_LONG) / u64::from(long_side))
            .expect("a resize-long dimension is at most 960"),
    );
    let input_height = align(
        u32::try_from(u64::from(source_height) * u64::from(RESIZE_LONG) / u64::from(long_side))
            .expect("a resize-long dimension is at most 960"),
    );
    let pixels = image.resize_exact(input_width, input_height, FilterType::Lanczos3).to_rgb8();
    let mut tensor = Array4::zeros((1, 3, input_height as usize, input_width as usize));
    for y in 0..input_height {
        for x in 0..input_width {
            let [r, g, b] = pixels.get_pixel(x, y).0;
            tensor[[0, 0, y as usize, x as usize]] = (f32::from(b) / 255.0 - 0.485) / 0.229;
            tensor[[0, 1, y as usize, x as usize]] = (f32::from(g) / 255.0 - 0.456) / 0.224;
            tensor[[0, 2, y as usize, x as usize]] = (f32::from(r) / 255.0 - 0.406) / 0.225;
        }
    }
    (tensor, input_width, input_height)
}

fn align(value: u32) -> u32 {
    value.div_ceil(ALIGNMENT) * ALIGNMENT
}

#[derive(Debug, Clone, Copy)]
struct OrientedRect {
    corners: [[f32; 2]; 4],
    width: f32,
    height: f32,
}

impl OrientedRect {
    fn short_side(self) -> f32 {
        self.width.min(self.height)
    }

    fn unclip(self, ratio: f32) -> Self {
        let perimeter = 2.0 * (self.width + self.height);
        if perimeter <= f32::EPSILON {
            return self;
        }
        let distance = self.width * self.height * ratio / perimeter;
        let center = self.corners.into_iter().fold([0.0; 2], |mut total, point| {
            total[0] += point[0] / 4.0;
            total[1] += point[1] / 4.0;
            total
        });
        let x_axis = unit(sub(self.corners[1], self.corners[0]));
        let y_axis = unit(sub(self.corners[3], self.corners[0]));
        let width = self.width + 2.0 * distance;
        let height = self.height + 2.0 * distance;
        Self { corners: rectangle_corners(center, x_axis, y_axis, width, height), width, height }
    }

    #[allow(clippy::cast_precision_loss)]
    fn to_original(self, map: &DetectionMap) -> [[f32; 2]; 4] {
        let mut points = self.corners.map(|[x, y]| {
            [
                (x / map.width as f32 * map.original_width as f32)
                    .clamp(0.0, map.original_width as f32),
                (y / map.height as f32 * map.original_height as f32)
                    .clamp(0.0, map.original_height as f32),
            ]
        });
        normalise_quad(&mut points);
        points
    }
}

/// Put an arbitrary rectangle's corners in clockwise, top-left-first order.
fn normalise_quad(points: &mut [[f32; 2]; 4]) {
    let center = points
        .into_iter()
        .fold([0.0; 2], |total, point| [total[0] + point[0] / 4.0, total[1] + point[1] / 4.0]);
    points.sort_unstable_by(|left, right| {
        (left[1] - center[1])
            .atan2(left[0] - center[0])
            .total_cmp(&(right[1] - center[1]).atan2(right[0] - center[0]))
    });
    let first = points
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| (left[0] + left[1]).total_cmp(&(right[0] + right[1])))
        .map_or(0, |(index, _)| index);
    points.rotate_left(first);
    if cross(points[0], points[1], points[2]) < 0.0 {
        points[1..].reverse();
    }
}

#[allow(clippy::cast_precision_loss)]
fn connected_components(map: &DetectionMap) -> Vec<Vec<[f32; 2]>> {
    let width = map.width as usize;
    let height = map.height as usize;
    let mut visited = vec![false; map.scores.len()];
    let mut components = Vec::new();
    for start in 0..map.scores.len() {
        if visited[start] || map.scores[start] <= BITMAP_THRESHOLD {
            continue;
        }
        let mut pixels = Vec::new();
        let mut queue = std::collections::VecDeque::from([start]);
        visited[start] = true;
        while let Some(index) = queue.pop_front() {
            let x = index % width;
            let y = index / width;
            pixels.push([x as f32 + 0.5, y as f32 + 0.5]);
            let left = x.saturating_sub(1);
            let right = (x + 1).min(width - 1);
            let top = y.saturating_sub(1);
            let bottom = (y + 1).min(height - 1);
            for neighbour_y in top..=bottom {
                for neighbour_x in left..=right {
                    let neighbour = neighbour_y * width + neighbour_x;
                    if !visited[neighbour] && map.scores[neighbour] > BITMAP_THRESHOLD {
                        visited[neighbour] = true;
                        queue.push_back(neighbour);
                    }
                }
            }
        }
        components.push(pixels);
    }
    components
}

fn minimum_area_rectangle(points: &[[f32; 2]]) -> Option<OrientedRect> {
    let hull = convex_hull(points);
    if hull.len() < 3 {
        return None;
    }
    let mut best: Option<(OrientedRect, f32)> = None;
    for (first, second) in hull.iter().zip(hull.iter().cycle().skip(1)).take(hull.len()) {
        let x_axis = unit(sub(*second, *first));
        if x_axis == [0.0, 0.0] {
            continue;
        }
        let y_axis = [-x_axis[1], x_axis[0]];
        let (mut min_x, mut max_x, mut min_y, mut max_y) =
            (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY);
        for point in &hull {
            let x = dot(*point, x_axis);
            let y = dot(*point, y_axis);
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        let width = max_x - min_x;
        let height = max_y - min_y;
        let area = width * height;
        let center =
            add(scale(x_axis, min_x.midpoint(max_x)), scale(y_axis, min_y.midpoint(max_y)));
        let rect = OrientedRect {
            corners: rectangle_corners(center, x_axis, y_axis, width, height),
            width,
            height,
        };
        if best.as_ref().is_none_or(|(_, best_area)| area < *best_area) {
            best = Some((rect, area));
        }
    }
    best.map(|(rect, _)| rect)
}

fn convex_hull(points: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let mut points = points.to_vec();
    points.sort_unstable_by(|left, right| {
        left[0].total_cmp(&right[0]).then(left[1].total_cmp(&right[1]))
    });
    points.dedup();
    let mut lower = Vec::new();
    for point in &points {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], *point) <= 0.0
        {
            lower.pop();
        }
        lower.push(*point);
    }
    let mut upper = Vec::new();
    for point in points.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], *point) <= 0.0
        {
            upper.pop();
        }
        upper.push(*point);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss, clippy::cast_sign_loss)]
fn mean_score(map: &DetectionMap, rect: &OrientedRect) -> f32 {
    let min_x =
        rect.corners.iter().map(|point| point[0]).fold(f32::INFINITY, f32::min).floor().max(0.0)
            as u32;
    let max_x = rect
        .corners
        .iter()
        .map(|point| point[0])
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .min(map.width as f32 - 1.0) as u32;
    let min_y =
        rect.corners.iter().map(|point| point[1]).fold(f32::INFINITY, f32::min).floor().max(0.0)
            as u32;
    let max_y = rect
        .corners
        .iter()
        .map(|point| point[1])
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .min(map.height as f32 - 1.0) as u32;
    let mut sum = 0.0;
    let mut count = 0_u32;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if point_in_convex_quad([x as f32 + 0.5, y as f32 + 0.5], rect.corners) {
                sum += map.scores[y as usize * map.width as usize + x as usize];
                count += 1;
            }
        }
    }
    if count == 0 { 0.0 } else { sum / count as f32 }
}

fn point_in_convex_quad(point: [f32; 2], corners: [[f32; 2]; 4]) -> bool {
    let mut is_positive = None;
    for (start, end) in corners.iter().zip(corners.iter().cycle().skip(1)).take(4) {
        let value = cross(*start, *end, point);
        if value.abs() <= f32::EPSILON {
            continue;
        }
        let next_is_positive = value.is_sign_positive();
        if is_positive.is_some_and(|positive| positive != next_is_positive) {
            return false;
        }
        is_positive = Some(next_is_positive);
    }
    true
}

fn rectangle_corners(
    center: [f32; 2],
    x_axis: [f32; 2],
    y_axis: [f32; 2],
    width: f32,
    height: f32,
) -> [[f32; 2]; 4] {
    let horizontal = scale(x_axis, width / 2.0);
    let vertical = scale(y_axis, height / 2.0);
    [
        sub(sub(center, horizontal), vertical),
        add(sub(center, vertical), horizontal),
        add(add(center, horizontal), vertical),
        add(sub(center, horizontal), vertical),
    ]
}

fn dot(left: [f32; 2], right: [f32; 2]) -> f32 {
    left[0] * right[0] + left[1] * right[1]
}
fn cross(origin: [f32; 2], left: [f32; 2], right: [f32; 2]) -> f32 {
    (left[0] - origin[0]) * (right[1] - origin[1]) - (left[1] - origin[1]) * (right[0] - origin[0])
}
fn add(left: [f32; 2], right: [f32; 2]) -> [f32; 2] {
    [left[0] + right[0], left[1] + right[1]]
}
fn sub(left: [f32; 2], right: [f32; 2]) -> [f32; 2] {
    [left[0] - right[0], left[1] - right[1]]
}
fn scale(point: [f32; 2], factor: f32) -> [f32; 2] {
    [point[0] * factor, point[1] * factor]
}
fn unit(point: [f32; 2]) -> [f32; 2] {
    let length = dot(point, point).sqrt();
    if length <= f32::EPSILON { [0.0, 0.0] } else { scale(point, 1.0 / length) }
}

#[cfg(test)]
mod tests {
    use image::{Rgb, RgbImage};

    use super::*;

    #[test]
    fn preprocessing_keeps_bgr_order_and_128_pixel_alignment() {
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(100, 50, Rgb([255, 127, 0])));
        let (tensor, width, height) = preprocess(&image);
        assert_eq!((width, height), (1024, 512));
        assert_eq!(tensor.shape(), &[1, 3, 512, 1024]);
        assert!((tensor[[0, 0, 0, 0]] + 2.117_904).abs() < 0.000_001);
        assert!((tensor[[0, 2, 0, 0]] - 2.64).abs() < 0.000_001);
    }

    #[test]
    fn db_postprocess_expands_and_maps_a_confident_region() {
        let mut scores = vec![0.0; 20 * 10];
        for y in 2..7 {
            for x in 4..15 {
                scores[y * 20 + x] = 0.9;
            }
        }
        let map = DetectionMap {
            scores,
            width: 20,
            height: 10,
            input_width: 20,
            input_height: 10,
            original_width: 200,
            original_height: 100,
        };
        let regions = map.regions();
        assert_eq!(regions.len(), 1);
        assert!(regions[0].confidence > 0.89);
        assert!(
            regions[0]
                .points
                .iter()
                .all(|[x, y]| *x >= 0.0 && *x <= 200.0 && *y >= 0.0 && *y <= 100.0)
        );
        assert!(cross(regions[0].points[0], regions[0].points[1], regions[0].points[2]) > 0.0);
    }

    #[test]
    #[ignore = "requires locally pulled PP-OCRv5 weights and ONNX Runtime"]
    fn installed_mobile_model_emits_a_real_score_map() {
        let config = aphrody_infer::SessionConfig::with_only(aphrody_models::Accelerator::Cpu);
        let mut detector = PpOcrDetector::load("ppocr-v5-mobile", &config).unwrap();
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(240, 48, Rgb([255, 255, 255])));
        let map = detector.detect(&image).unwrap();
        assert_eq!(map.scores.len(), map.width as usize * map.height as usize);
        assert!(map.scores.iter().all(|score| score.is_finite()));
    }
}
