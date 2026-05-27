# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Aphrody Blender extension entry point.

Installed as a Blender 4.2+/5.1 *extension* (folder with
``blender_manifest.toml``), this re-exports the add-on's registration hooks from
:mod:`aphrody_addon`. For a legacy single-file install, install
``aphrody_addon.py`` directly via *Install from Disk*.
"""

from .aphrody_addon import bl_info, register, unregister

__all__ = ["bl_info", "register", "unregister"]
