import { TextField, Select, MenuItem } from "@mui/material";

export function Form() {
  return (
    <form>
      <TextField label="Nom" value={name} onChange={(e, val) => setName(val)} />
      <TextField variant="outlined" label="Email" />
      <Select variant="outlined" value={x} onChange={(e) => setX(e.target.value)}>
        <MenuItem value="a">A</MenuItem>
        <MenuItem value="b">B</MenuItem>
      </Select>
    </form>
  );
}
