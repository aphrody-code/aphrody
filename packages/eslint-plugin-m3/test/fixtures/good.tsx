// Fixture conforme — ne doit déclencher AUCUNE règle m3.
import { MdFilledButton, MdIcon, MdIconButton, MdSwitch, MdTooltip } from "@aphrody/m3-react";

export function Good() {
  return (
    <div>
      <MdIcon>delete</MdIcon>
      <md-icon>home</md-icon>
      <MdIcon style={{ "--md-icon-fill": 1 }}>favorite</MdIcon>
      <MdFilledButton className="m-2">X</MdFilledButton>
      <MdFilledButton style={{ color: "var(--md-sys-color-on-surface-variant)" }}>Y</MdFilledButton>
      <MdSwitch selected />
      <MdTooltip text="hi" />
      <MdIconButton aria-label="Supprimer">
        <MdIcon>delete</MdIcon>
      </MdIconButton>
    </div>
  );
}
