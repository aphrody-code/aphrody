"use client";
import "@aphrody-code/material-web/icon/icon.js";
import { GitHub } from "@mui/icons-material";

export function Demo() {
  return (
    <div>
      {/* MIGRATION-TODO: icone Close: props MUI (fontSize/color/sx) retirees -> piloter via --md-icon-size / --md-icon-fill / currentColor. */}
      <md-icon>close</md-icon>
      <md-icon>emoji_events</md-icon>
      <md-icon>delete</md-icon>
      {/* MIGRATION-TODO: icone GitHub: logo de marque absent de Material Symbols -> garder en SVG (set de marque dedie). Slug suggere: github. */}
      <GitHub />
      <md-icon>brightness_4</md-icon>
    </div>
  );
}
