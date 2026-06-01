import {
  MdFilledTextField,
  MdMenuItem,
  MdOutlinedSelect,
  MdOutlinedTextField,
} from "@aphrody-code/m3-react";

export function Form() {
  return (
    <form>
      {/* MIGRATION-TODO: onChange(e, value) MUI -> material-web emet (input/change) natifs : lire e.target.value (le 2e parametre disparait). */}
      <MdFilledTextField label="Nom" value={name} onChange={(e, val) => setName(val)} />
      <MdOutlinedTextField label="Email" />
      <MdOutlinedSelect value={x} onChange={(e) => setX(e.target.value)}>
        <MdMenuItem value="a">A</MdMenuItem>
        <MdMenuItem value="b">B</MdMenuItem>
      </MdOutlinedSelect>
    </form>
  );
}
