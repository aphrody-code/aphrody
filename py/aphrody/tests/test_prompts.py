# SPDX-License-Identifier: Apache-2.0
"""Tests for the dependency-free :mod:`aphrody.prompts` library."""

from __future__ import annotations

import pytest

from aphrody import prompts


def test_aspect_ratios_and_sizes() -> None:
    assert len(prompts.ASPECT_RATIOS) == 10
    assert "16:9" in prompts.ASPECT_RATIOS
    assert prompts.IMAGE_SIZES == ("1K", "2K", "4K")
    assert prompts.MAX_REFERENCE_IMAGES == 14


def test_render_template_fills_known_and_keeps_unknown() -> None:
    out = prompts.render_template("logo", brand_name="Aphrody")
    assert '"Aphrody"' in out
    # Unknown placeholders are preserved verbatim, never raising.
    assert "{industry}" in out


def test_render_template_unknown_id_raises() -> None:
    with pytest.raises(KeyError):
        prompts.render_template("does-not-exist")


def test_all_templates_render_without_crashing() -> None:
    for tmpl in prompts.list_templates():
        rendered = tmpl.render()
        assert isinstance(rendered, str)
        assert rendered
        # Every declared placeholder is parsed from the template body.
        for name in tmpl.placeholders:
            assert "{" + name + "}" in tmpl.template


def test_list_templates_category_filter() -> None:
    products = prompts.list_templates("product")
    assert products
    assert all(t.category == "product" for t in products)


def test_enhance_prompt_preset_appends_modifiers() -> None:
    out = prompts.enhance_prompt("a cat on a sofa", preset="photoreal")
    assert out.startswith("a cat on a sofa.")
    assert "85mm" in out
    assert "4K" in out


def test_enhance_prompt_explicit_overrides_preset() -> None:
    out = prompts.enhance_prompt(
        "x", preset="photoreal", lens="200mm telephoto", quality=None
    )
    assert "200mm telephoto" in out
    assert "85mm" not in out


def test_enhance_prompt_unknown_preset_raises() -> None:
    with pytest.raises(ValueError, match="unknown preset"):
        prompts.enhance_prompt("x", preset="nope")


def test_enhance_prompt_negatives_clause() -> None:
    out = prompts.enhance_prompt("x", negatives=True)
    assert "Avoid:" in out


def test_quote_text_is_idempotent() -> None:
    assert prompts.quote_text("Open Now") == '"Open Now"'
    assert prompts.quote_text('"Open Now"') == '"Open Now"'


def test_build_negatives_default_and_custom() -> None:
    assert prompts.build_negatives("blur", "extra fingers").startswith("Avoid:")
    assert "blur" in prompts.build_negatives()  # default set
