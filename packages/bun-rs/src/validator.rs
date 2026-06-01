// SPDX-License-Identifier: Apache-2.0

use crate::symbols::MATERIAL_SYMBOLS;

pub struct Issue {
	pub level: &'static str, // "error" or "warning"
	pub rule: &'static str,
	pub message: String,
	pub line: usize,
	pub matched: String,
}

const M3_ROLES: &[&str] = &[
	"background",
	"on-background",
	"surface",
	"surface-dim",
	"surface-bright",
	"surface-container-lowest",
	"surface-container-low",
	"surface-container",
	"surface-container-high",
	"surface-container-highest",
	"on-surface",
	"surface-variant",
	"on-surface-variant",
	"inverse-surface",
	"inverse-on-surface",
	"outline",
	"outline-variant",
	"shadow",
	"scrim",
	"surface-tint",
	"primary",
	"on-primary",
	"primary-container",
	"on-primary-container",
	"inverse-primary",
	"secondary",
	"on-secondary",
	"secondary-container",
	"on-secondary-container",
	"tertiary",
	"on-tertiary",
	"tertiary-container",
	"on-tertiary-container",
	"error",
	"on-error",
	"error-container",
	"on-error-container",
	"primary-fixed",
	"primary-fixed-dim",
	"on-primary-fixed",
	"on-primary-fixed-variant",
	"secondary-fixed",
	"secondary-fixed-dim",
	"on-secondary-fixed",
	"on-secondary-fixed-variant",
	"tertiary-fixed",
	"tertiary-fixed-dim",
	"on-tertiary-fixed",
	"on-tertiary-fixed-variant",
];

fn find_line_number(content: &str, byte_offset: usize) -> usize {
	let mut line = 1;
	for (i, c) in content.char_indices() {
		if i >= byte_offset {
			break;
		}
		if c == '\n' {
			line += 1;
		}
	}
	line
}

pub fn validate_m3_spec(code: &str) -> (i32, Vec<Issue>) {
	let mut issues = Vec::new();

	// 1. Line-by-line checks: Hardcoded colors & Color roles
	let mut line_num = 1;
	for line in code.lines() {
		// A. Hardcoded color checks
		let has_style_keyword = line.contains("color")
			|| line.contains("background")
			|| line.contains("border")
			|| line.contains("style")
			|| line.contains("theme")
			|| line.contains("palette")
			|| line.contains("fill")
			|| line.contains("stroke");

		if has_style_keyword {
			let mut chars = line.char_indices().peekable();
			while let Some((i, c)) = chars.next() {
				if c == '#' {
					let start = i;
					let mut end = i + 1;
					while let Some(&(next_idx, next_char)) = chars.peek() {
						if next_char.is_ascii_hexdigit() {
							end = next_idx + next_char.len_utf8();
							chars.next();
						} else {
							break;
						}
					}
					let len = end - (start + 1);
					if len == 3 || len == 4 || len == 6 || len == 8 {
						let hex = &line[start..end];
						let hex_lower = hex.to_lowercase();
						// Ignore standard baseline seed and default black/white shorthand if needed, but flag custom ones
						if hex_lower != "#6750a4" && hex_lower != "#000000" && hex_lower != "#ffffff" && hex_lower != "#000" && hex_lower != "#fff" {
							issues.push(Issue {
								level: "warning",
								rule: "m3/no-hardcoded-color",
								message: format!("Hardcoded color '{}' found. Recommending M3 theme tokens (e.g. var(--md-sys-color-*)) instead.", hex),
								line: line_num,
								matched: hex.to_string(),
							});
						}
					}
				}
			}
		}

		// B. Color roles validation
		let mut search_idx = 0;
		while let Some(pos) = line[search_idx..].find("--md-sys-color-") {
			let start = search_idx + pos;
			let role_start = start + 15;
			let mut end = role_start;
			let bytes = line.as_bytes();
			while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'-') {
				end += 1;
			}
			if end > role_start {
				let role = &line[role_start..end];
				if !M3_ROLES.contains(&role) {
					issues.push(Issue {
						level: "error",
						rule: "m3/valid-color-role",
						message: format!("Invalid Material 3 color role: '--md-sys-color-{}'. Expected one of the 49 standard system roles.", role),
						line: line_num,
						matched: format!("--md-sys-color-{}", role),
					});
				}
			}
			search_idx = end;
		}

		line_num += 1;
	}

	// 2. Global checks: md-icon validations
	let mut search_idx = 0;
	while let Some(pos) = code[search_idx..].find("<md-icon") {
		let start = search_idx + pos;
		let next_char = code.as_bytes().get(start + 8);
		if next_char == Some(&b' ') || next_char == Some(&b'>') {
			// Find closing tag bracket '>'
			if let Some(close_bracket) = code[start..].find('>') {
				let tag_end = start + close_bracket;
				// Find closing tag '</md-icon>'
				if let Some(close_tag) = code[tag_end..].find("</md-icon>") {
					let text_start = tag_end + 1;
					let text_end = tag_end + close_tag;
					let icon_name = code[text_start..text_end].trim();
					
					// If it's a literal name and not empty or dynamic (curly braces)
					if !icon_name.is_empty() && !icon_name.starts_with('{') && !icon_name.contains('$') && !icon_name.contains('<') {
						if MATERIAL_SYMBOLS.binary_search(&icon_name).is_err() {
							let line = find_line_number(code, text_start);
							issues.push(Issue {
								level: "error",
								rule: "m3/valid-icon-name",
								message: format!("Invalid Material Symbol icon name: '{}'. This name does not match any of the 4,253 official Google glyphs.", icon_name),
								line,
								matched: icon_name.to_string(),
							});
						}
					}
					search_idx = text_end + 10;
					continue;
				}
			}
		}
		search_idx = start + 8;
	}

	// 3. Motion cubic-bezier checks
	let mut search_idx = 0;
	while let Some(pos) = code[search_idx..].find("cubic-bezier") {
		let start = search_idx + pos;
		if let Some(open_paren) = code[start..].find('(') {
			let param_start = start + open_paren + 1;
			if let Some(close_paren) = code[param_start..].find(')') {
				let param_end = param_start + close_paren;
				let params_str = &code[param_start..param_end];
				let parts: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();
				if parts.len() == 4 {
					let p0 = parts[0].parse::<f32>().unwrap_or(-1.0);
					let p1 = parts[1].parse::<f32>().unwrap_or(-1.0);
					let p2 = parts[2].parse::<f32>().unwrap_or(-1.0);
					let p3 = parts[3].parse::<f32>().unwrap_or(-1.0);

					// Standard curves definitions (M3, M2 legacy, and Expressive presets)
					let presets = [
						("Emphasized / Standard", (0.20_f32, 0.00_f32, 0.00_f32, 1.00_f32)),
						("Emphasized Decelerate", (0.05, 0.70, 0.10, 1.00)),
						("Emphasized Accelerate", (0.30, 0.00, 0.80, 0.15)),
						("Standard Decelerate", (0.00, 0.00, 0.00, 1.00)),
						("Standard Accelerate", (0.30, 0.00, 1.00, 1.00)),
						("Linear", (0.00, 0.00, 1.00, 1.00)),
						("Legacy (M2)", (0.40, 0.00, 0.20, 1.00)),
						("Legacy Decelerate (M2)", (0.00, 0.00, 0.20, 1.00)),
						("Legacy Accelerate (M2)", (0.40, 0.00, 1.00, 1.00)),
						("Fast Spatial", (0.42, 1.67, 0.21, 0.90)),
						("Default Spatial", (0.38, 1.21, 0.22, 1.00)),
						("Fast Effects", (0.31, 0.94, 0.34, 1.00)),
						("Default Effects", (0.34, 0.80, 0.34, 1.00)),
					];

					let mut matched = false;
					for (_name, curve) in &presets {
						if (p0 - curve.0).abs() < 0.02
							&& (p1 - curve.1).abs() < 0.02
							&& (p2 - curve.2).abs() < 0.02
							&& (p3 - curve.3).abs() < 0.02
						{
							matched = true;
							break;
						}
					}

					if !matched {
						let line = find_line_number(code, start);
						issues.push(Issue {
							level: "warning",
							rule: "m3/motion-curves",
							message: format!("Custom cubic-bezier({}) found. Recommending using canonical M3 curves (e.g. Emphasized, Standard) or spatial springs for consistent physics-based animation.", params_str),
							line,
							matched: format!("cubic-bezier({})", params_str),
						});
					}
				}
				search_idx = param_end + 1;
				continue;
			}
		}
		search_idx = start + 12;
	}

	// 4. Accessibility Interactive labeling checks
	let interactive_tags = ["<md-icon-button", "<md-fab", "<md-branded-fab", "<md-filled-icon-button"];
	for tag in &interactive_tags {
		let mut search_idx = 0;
		while let Some(pos) = code[search_idx..].find(tag) {
			let start = search_idx + pos;
			if let Some(close_bracket) = code[start..].find('>') {
				let tag_end = start + close_bracket;
				let tag_attrs = &code[start..tag_end];
				
				let has_label = tag_attrs.contains("aria-label=")
					|| tag_attrs.contains("label=")
					|| tag_attrs.contains("aria-labelledby=");

				if !has_label {
					let line = find_line_number(code, start);
					issues.push(Issue {
						level: "warning",
						rule: "m3/require-icon-button-label",
						message: format!("Interactive component '{}' is missing a screen-reader labeling attribute ('aria-label' or 'label').", tag.trim_start_matches('<')),
						line,
						matched: tag_attrs.to_string(),
					});
				}
				search_idx = tag_end + 1;
			} else {
				search_idx = start + tag.len();
			}
		}
	}

	// Compute score: starts at 100
	let mut score = 100;
	for issue in &issues {
		if issue.level == "error" {
			score -= 5;
		} else {
			score -= 2;
		}
	}
	score = score.max(0);

	(score, issues)
}
