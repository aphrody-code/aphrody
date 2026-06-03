// Fixture VOLONTAIREMENT non conforme — doit déclencher chaque règle m3.
import Button from "@mui/material/Button"; // no-mui-import
import { Delete } from "@mui/icons-material"; // no-mui-import
import { MdFilledButton, MdIcon, MdIconButton, MdSwitch, MdTooltip } from "@aphrody/m3-react";

export function Bad() {
  return (
    <div>
      <MdIcon>Delete</MdIcon>
      {/* valid-icon-name : PascalCase -> delete */}
      <md-icon>notaglyphname</md-icon>
      {/* valid-icon-name : introuvable */}
      <MdIcon style={{ fontVariationSettings: "'FILL' 1" }}>home</MdIcon>
      {/* prefer-icon-token */}
      <MdFilledButton sx={{ m: 1 }}>X</MdFilledButton>
      {/* no-sx-prop */}
      <MdFilledButton style={{ color: "#ff0000" }}>Y</MdFilledButton>
      {/* no-hardcoded-color */}
      <MdFilledButton style={{ color: "var(--md-sys-color-primry)" }}>Z</MdFilledButton>
      {/* valid-color-role : primry -> primary */}
      <MdSwitch checked />
      {/* no-mui-prop-on-md : checked -> selected */}
      <MdTooltip title="hi" />
      {/* no-mui-prop-on-md : title -> text */}
      <MdIconButton>
        <MdIcon>delete</MdIcon>
      </MdIconButton>
      {/* require-icon-button-label : pas d'aria-label */}
      <Button>mui</Button>
      {Delete}
    </div>
  );
}
