# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Nano Banana Pro prompt library, modifier vocabulary and enhancer.

This module is **pure Python with no third-party dependencies** — it encodes
Google's official prompting guidance for the Gemini 3 Pro Image model
(*Nano Banana Pro*) as reusable templates plus a programmatic prompt enhancer.

It is consumed by :mod:`aphrody.images` (generation) and the ``aphrody image``
CLI, but is equally usable standalone:

    >>> from aphrody.prompts import render_template, enhance_prompt
    >>> render_template("logo", brand_name="Aphrody", industry="developer tools",
    ...                 logo_concept="abstract orbit", font_style="geometric sans",
    ...                 color_palette="indigo and white")  # doctest: +ELLIPSIS
    'A minimalist vector logo for "Aphrody"...'
    >>> enhance_prompt("a cat on a sofa", preset="photoreal")  # doctest: +ELLIPSIS
    'a cat on a sofa. ...85mm...4K...'

Sources distilled into this module (Google + community guidance):
- https://cloud.google.com/blog/products/ai-machine-learning/ultimate-prompting-guide-for-nano-banana
- https://blog.google/products-and-platforms/products/gemini/prompting-tips-nano-banana-pro/
- https://github.com/ZeroLu/awesome-nanobanana-pro
"""

from __future__ import annotations

import string
from dataclasses import dataclass, field

# ---------------------------------------------------------------------------
# Hard model facts (Nano Banana Pro / gemini-3-pro-image-preview)
# ---------------------------------------------------------------------------

#: The ten aspect ratios accepted by the model's ``ImageConfig.aspect_ratio``.
ASPECT_RATIOS: tuple[str, ...] = (
    "1:1",
    "2:3",
    "3:2",
    "3:4",
    "4:3",
    "4:5",
    "5:4",
    "9:16",
    "16:9",
    "21:9",
)

#: The three native resolution tiers (``ImageConfig.image_size``).
IMAGE_SIZES: tuple[str, ...] = ("1K", "2K", "4K")

#: Maximum number of reference images accepted per request (API ceiling).
MAX_REFERENCE_IMAGES = 14

# ---------------------------------------------------------------------------
# Modifier vocabulary — concrete tokens for the enhancer / CLI completion
# ---------------------------------------------------------------------------

#: Lens / focal-length descriptors (camera realism).
LENSES: tuple[str, ...] = (
    "85mm f/1.4 portrait lens with shallow depth of field",
    "50mm prime lens",
    "35mm full-frame lens",
    "24mm wide-angle lens",
    "macro lens with extreme close-up detail",
    "telephoto lens with compressed perspective",
    "low-angle shot with shallow depth of field (f/1.8)",
)

#: Lighting setups.
LIGHTING: tuple[str, ...] = (
    "three-point softbox studio lighting",
    "soft key light with a subtle rim light",
    "Rembrandt side light",
    "dramatic chiaroscuro high-contrast lighting",
    "golden-hour backlighting with long shadows",
    "natural window light from the side",
    "bright airy diffused studio light",
    "dramatic spotlight against a dark background",
)

#: Camera bodies / film looks.
CAMERAS: tuple[str, ...] = (
    "Arri Alexa cinematic look",
    "Hasselblad medium-format detail",
    "Kodak Portra 400 film grain",
    "medium-format analog film, high saturation",
    "Fujifilm color science",
)

#: Quality / resolution intensifiers (textual; the real tier is image_size).
QUALITY: tuple[str, ...] = (
    "4K",
    "ultra-detailed textures",
    "ultra-realistic",
    "high dynamic range",
    "photoreal textures",
    "crisp detail",
    "commercial grade",
)

#: Common negative constraints. Google prefers POSITIVE framing — these are a
#: community convention, applied only when explicitly requested.
NEGATIVES: tuple[str, ...] = (
    "no blur",
    "no duplicated objects",
    "no warped or garbled text",
    "no extra fingers or limbs",
    "no watermark",
    "no background distractions",
    "no oversaturation",
)


@dataclass(frozen=True)
class PromptTemplate:
    """A reusable Nano Banana Pro prompt with ``{placeholder}`` slots.

    Attributes:
        id: Short kebab-case identifier.
        category: Broad use-case category.
        template: The template string with ``str.format`` style placeholders.
    """

    id: str
    category: str
    template: str
    #: Placeholder names parsed out of ``template`` (lazily computed).
    placeholders: tuple[str, ...] = field(default_factory=tuple)

    def render(self, **values: str) -> str:
        """Fill the template, leaving unknown placeholders intact.

        Args:
            **values: Placeholder substitutions.

        Returns:
            The rendered prompt string. Missing placeholders are preserved
            verbatim (e.g. ``{subject}``) so partial rendering never raises.
        """
        return _SafeFormatter().vformat(self.template, (), dict(values))


class _SafeFormatter(string.Formatter):
    """``str.format`` that leaves missing keys as ``{key}`` instead of raising."""

    def get_value(self, key: object, args: object, kwargs: object) -> object:  # type: ignore[override]
        if isinstance(key, str):
            return kwargs.get(key, "{" + key + "}")  # type: ignore[union-attr]
        return super().get_value(key, args, kwargs)  # type: ignore[arg-type]


def _placeholders(template: str) -> tuple[str, ...]:
    """Return the ordered, de-duplicated placeholder names in *template*."""
    seen: list[str] = []
    for _lit, name, _spec, _conv in string.Formatter().parse(template):
        if name and name not in seen:
            seen.append(name)
    return tuple(seen)


def _t(id_: str, category: str, template: str) -> PromptTemplate:
    """Build a :class:`PromptTemplate` with placeholders parsed from *template*."""
    return PromptTemplate(
        id=id_,
        category=category,
        template=template.strip(),
        placeholders=_placeholders(template),
    )


# ---------------------------------------------------------------------------
# Template catalogue — 20 production templates spanning Nano Banana Pro's
# strongest use-cases. Each embeds real best-practice modifiers.
# ---------------------------------------------------------------------------

_TEMPLATES: tuple[PromptTemplate, ...] = (
    _t(
        "photorealistic-portrait",
        "portrait",
        "A professional studio headshot of {subject}, {expression} expression, "
        "looking {gaze_direction}. Shot on an 85mm f/1.4 lens with shallow depth "
        "of field and flattering portrait compression, exquisite focus on the "
        "eyes. Three-point lighting with a soft key light and subtle rim light "
        "against a {backdrop_color} seamless backdrop. Polished contemporary "
        "color grading, ultra-realistic skin texture, 4K.",
    ),
    _t(
        "golden-hour-portrait",
        "portrait",
        "An emotional film-style portrait of {subject} in {setting}, "
        "{expression}. Shot on Kodak Portra 400, 50mm lens, golden-hour sunset "
        "backlighting with warm rim light, subtle film grain, shallow depth of "
        "field. Cinematic, dreamy, storytelling mood, 4K.",
    ),
    _t(
        "product-mockup",
        "product",
        "Place this {product} on a {surface} with {lighting_setup}. Medium "
        "close-up, center-framed, crisp contact shadow and shallow depth of "
        "field (f/2.8). Glossy highlights reveal the {material} surface. Clean "
        "ecommerce-ready commercial product photography, high dynamic range, "
        "ultra-detailed textures, 4K.",
    ),
    _t(
        "ecommerce-white-bg",
        "product",
        "Create a clean ecommerce-ready pure-white background product shot of "
        "this {product}. Centered, even softbox studio lighting (three-point), "
        "crisp natural contact shadow, true-to-life color, ultra-detailed "
        "textures revealing {material}, 4K.",
    ),
    _t(
        "luxury-product-hero",
        "product",
        "Turn this {product} into a luxury hero shot floating on {environment} "
        "with suspended particles and a {backdrop} backdrop. Dramatic spotlight, "
        "golden-hour glow, glossy chrome reflections, smoky gradient atmosphere. "
        "Ethereal high-end commercial advertisement, ultra-detailed, 4K.",
    ),
    _t(
        "typography-poster",
        "typography",
        'A {orientation} poster with the headline "{headline_text}" rendered in '
        '{font_style} at the {text_position}, and the subtitle "{subtitle_text}" '
        "below it. {color_palette} color scheme, balanced negative space, "
        "high-contrast layout. Crisp vector-clean typography, print-ready, 2K.",
    ),
    _t(
        "text-heavy-poster",
        "typography",
        'A {orientation} event poster for "{event_name}". Title "{title_text}" '
        'in large {font_style} at the top, supporting line "{subtitle_text}" in '
        'the middle, and details "{details_text}" at the bottom — keep every '
        "block of text within 1-6 words. {color_palette} palette, strong grid "
        "layout, balanced hierarchy. Print-ready, crisp typography, 4K.",
    ),
    _t(
        "logo",
        "branding",
        'A minimalist vector logo for "{brand_name}", a {industry} brand. '
        "{logo_concept} icon paired with a clean {font_style} wordmark. "
        "{color_palette} palette, flat design, centered on a plain white "
        "background with generous clear space. Sharp edges, scalable, "
        "high-resolution, 1:1.",
    ),
    _t(
        "brand-identity-kit",
        "branding",
        'Generate realistic brand mockups for the "{brand_name}" logo applied '
        "to {mockup_surfaces}. Cohesive {color_palette} palette and {font_style} "
        "typography, soft studio lighting with realistic material reflections "
        "and shadows. High-end commercial mockup photography, ultra-detailed, 4K.",
    ),
    _t(
        "cinematic-scene",
        "cinematic",
        "A cinematic {shot_type} of {subject} in {setting}. {camera_action}, "
        "anamorphic framing, Arri Alexa look with cinematic color grading and "
        "muted teal tones. {lighting} creating long shadows, atmospheric haze, "
        "shallow depth of field (f/1.8). Photoreal textures, film grain, 4K.",
    ),
    _t(
        "infographic-diagram",
        "infographic",
        'A clean, modern {layout_style} infographic titled "{title}" '
        "illustrating {topic}. {section_count} sections, each with a flat icon, "
        "a bold heading, and one line of body text. White background, thin grey "
        "dividers, {accent_color} accent color, consistent sans-serif "
        "typography, generous spacing. Flat vector style, crisp labels, 2K.",
    ),
    _t(
        "data-visualization",
        "infographic",
        "A data chart visualizing {dataset} as a {chart_type}. Title "
        '"{chart_title}", clearly readable axis labels and value labels '
        "({example_values}), consistent label styling. {color_palette} palette, "
        "gridlines, white background, flat editorial dashboard style. Crisp "
        "legible typography, 2K.",
    ),
    _t(
        "sticker-icon",
        "illustration",
        "A die-cut sticker of {subject} in a cute kawaii style with bold clean "
        "outlines, a thick white border, and flat cel-shaded coloring. "
        "{color_palette} palette, subtle drop shadow, isolated on a plain white "
        "background. Glossy vinyl finish, high-resolution, 1:1.",
    ),
    _t(
        "3d-render",
        "render",
        "A 3D isometric diorama of {subject}, cute polished 3D render style "
        "with rounded edges, soft global-illumination studio lighting and "
        "realistic {material} materials. Soft ambient occlusion, subtle depth "
        "of field, void background. Octane-style render, ultra-detailed, 4K.",
    ),
    _t(
        "split-realism-wireframe",
        "render",
        "A split-view render of a single {object}: the left half in full "
        "photorealism, the right half a hard-cut wireframe interior. Render only "
        "ONE object in the entire frame. Wireframe is white (~80%) with "
        "{accent_color} accents (~20%), void background, even studio lighting. "
        "Ultra-detailed, 4K.",
    ),
    _t(
        "architecture",
        "architecture",
        "An architectural visualization of a {building_type} in "
        "{architectural_style} style, {exterior_materials} facade. "
        "Three-quarter exterior view, 24mm wide-angle lens, {time_of_day} "
        "lighting with realistic sky and long shadows. Photoreal V-Ray render, "
        "crisp material detail, high dynamic range, 4K.",
    ),
    _t(
        "floorplan-to-interior",
        "architecture",
        "Convert this floor plan into a photorealistic interior render of the "
        "{room_type}. {design_style} interior, {material_palette} materials and "
        "furnishings, natural daylight from the windows with soft shadows, "
        "eye-level wide-angle view (24mm). Photoreal architectural render, crisp "
        "material detail, 4K.",
    ),
    _t(
        "food",
        "food",
        "A {shot_type} food photograph of {dish} on {tableware}, garnished with "
        "{garnish}. Soft natural window light from the side, shallow depth of "
        "field with macro detail on the texture, matching props and {surface} "
        "surface. Appetizing commercial food styling, glossy highlights, "
        "ultra-detailed, 4K.",
    ),
    _t(
        "fashion-editorial",
        "fashion",
        "A fashion magazine editorial of {model_description} wearing {outfit}, "
        "posing with a confident statuesque stance, slightly turned. "
        "Medium-full shot, center-framed, against a {backdrop_color} studio "
        "backdrop. Shot on medium-format analog film, pronounced grain, high "
        "saturation, cinematic lighting. High-end editorial style, 4K.",
    ),
    _t(
        "character-sheet",
        "character",
        "A character reference sheet for {character_name}, a "
        "{character_description}. Front, side and three-quarter turnaround views "
        "on a single sheet, plus a head close-up. Keep the character 100% "
        "consistent across every view — identical face, outfit and proportions. "
        "Neutral grey background, even diffuse studio lighting, {art_style} "
        "style, ultra-detailed, 4K.",
    ),
    _t(
        "ui-mockup",
        "ui",
        "Design a clean, modern {screen_type} for a {product_type} app. "
        "Minimalist layout with a full-width hero section, brand logo top-left, "
        "a thin navigation bar, and {key_components}. Use {font_pairing} "
        "typography and a {color_palette} palette with consistent spacing and a "
        "clear visual hierarchy. Crisp UI, pixel-sharp text, 2K.",
    ),
)

#: Catalogue keyed by template id.
TEMPLATES: dict[str, PromptTemplate] = {t.id: t for t in _TEMPLATES}


# ---------------------------------------------------------------------------
# Style presets for the enhancer
# ---------------------------------------------------------------------------

#: Named bundles of modifiers appended by :func:`enhance_prompt`.
STYLE_PRESETS: dict[str, dict[str, str]] = {
    "photoreal": {
        "lens": "85mm f/1.4 lens with shallow depth of field",
        "lighting": "three-point softbox studio lighting",
        "camera": "Hasselblad medium-format detail",
        "quality": "ultra-realistic skin and material texture, 4K",
    },
    "cinematic": {
        "lens": "anamorphic 35mm lens (f/1.8)",
        "lighting": "dramatic chiaroscuro lighting with long shadows",
        "camera": "Arri Alexa cinematic look, muted teal color grade",
        "quality": "photoreal textures, film grain, 4K",
    },
    "product": {
        "lens": "50mm lens, f/2.8, center-framed",
        "lighting": "even softbox studio lighting with crisp contact shadow",
        "camera": "commercial product photography",
        "quality": "ultra-detailed textures, high dynamic range, 4K",
    },
    "studio": {
        "lens": "85mm portrait lens",
        "lighting": "soft key light with subtle rim light, seamless backdrop",
        "camera": "studio photography",
        "quality": "crisp detail, 4K",
    },
    "illustration": {
        "lighting": "flat even lighting",
        "camera": "clean vector illustration",
        "quality": "bold clean outlines, high-resolution",
    },
    "render": {
        "lighting": "soft global illumination with ambient occlusion",
        "camera": "Octane-style 3D render",
        "quality": "ultra-detailed materials, 4K",
    },
}


def list_templates(category: str | None = None) -> list[PromptTemplate]:
    """Return all templates, optionally filtered by *category*.

    Args:
        category: When given, only templates with this exact category.

    Returns:
        A list of :class:`PromptTemplate` in catalogue order.
    """
    items = list(_TEMPLATES)
    if category:
        items = [t for t in items if t.category == category]
    return items


def get_template(template_id: str) -> PromptTemplate:
    """Look up a template by id.

    Args:
        template_id: The template's kebab-case id.

    Returns:
        The matching :class:`PromptTemplate`.

    Raises:
        KeyError: If no template has that id.
    """
    try:
        return TEMPLATES[template_id]
    except KeyError:
        raise KeyError(
            f"unknown template {template_id!r}; available: "
            f"{', '.join(sorted(TEMPLATES))}"
        ) from None


def render_template(template_id: str, **values: str) -> str:
    """Render template *template_id* with the supplied placeholder *values*.

    Args:
        template_id: The template id.
        **values: Placeholder substitutions; missing ones are left as
            ``{placeholder}`` so the result is always usable.

    Returns:
        The rendered prompt string.

    Raises:
        KeyError: If *template_id* is unknown.
    """
    return get_template(template_id).render(**values)


def quote_text(text: str) -> str:
    """Wrap literal on-image *text* in double quotes per Google guidance.

    Nano Banana Pro renders the most accurate typography when the exact words
    are quoted. Keeps text short (1-6 words renders best).

    Args:
        text: The literal words to render on the image.

    Returns:
        The text wrapped in double quotes (idempotent if already quoted).
    """
    text = text.strip()
    if len(text) >= 2 and text[0] == '"' and text[-1] == '"':
        return text
    return f'"{text}"'


def build_negatives(*terms: str) -> str:
    """Build a constraint clause from negative *terms*.

    Args:
        *terms: Things to avoid (without a leading "no"). When empty, a curated
            default set (:data:`NEGATIVES`) is used.

    Returns:
        A clause like ``"Avoid: blur, warped text."`` suitable for appending.
    """
    chosen = (
        list(terms) if terms else [n.removeprefix("no ") for n in NEGATIVES]
    )
    return "Avoid: " + ", ".join(chosen) + "."


def enhance_prompt(
    prompt: str,
    *,
    preset: str | None = None,
    lens: str | None = None,
    lighting: str | None = None,
    camera: str | None = None,
    quality: str | None = "4K",
    negatives: bool = False,
) -> str:
    """Append best-practice Nano Banana Pro modifiers to a base *prompt*.

    The base prompt is preserved verbatim; modifiers are appended as a single
    descriptive sentence. A *preset* (see :data:`STYLE_PRESETS`) supplies
    sensible defaults that explicit keyword arguments override.

    Args:
        prompt: The user's base description.
        preset: A named style preset (e.g. ``"photoreal"``, ``"cinematic"``).
        lens: Explicit lens descriptor (overrides the preset).
        lighting: Explicit lighting descriptor.
        camera: Explicit camera / film-look descriptor.
        quality: Quality/resolution suffix; pass ``None`` to omit.
        negatives: When ``True``, append a default negative-constraint clause.

    Returns:
        The enhanced prompt string.

    Raises:
        ValueError: If *preset* is given but unknown.
    """
    base = prompt.strip().rstrip(".")
    bundle: dict[str, str] = {}
    if preset is not None:
        if preset not in STYLE_PRESETS:
            raise ValueError(
                f"unknown preset {preset!r}; available: "
                f"{', '.join(sorted(STYLE_PRESETS))}"
            )
        bundle = dict(STYLE_PRESETS[preset])

    resolved = {
        "lens": lens or bundle.get("lens"),
        "lighting": lighting or bundle.get("lighting"),
        "camera": camera or bundle.get("camera"),
        "quality": quality if quality is not None else bundle.get("quality"),
    }
    modifiers = [v for v in resolved.values() if v]

    parts = [base + "."]
    if modifiers:
        parts.append(" ".join(m.rstrip(".") + "." for m in modifiers))
    if negatives:
        parts.append(build_negatives())
    return " ".join(parts)
