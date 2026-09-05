import Checkbox from "@mui/material/Checkbox";
import { Switch } from "@mui/material";

export function Toggles() {
  return (
    <div>
      <Checkbox checked={ok} onChange={(e) => setOk(e.target.checked)} />
      <Switch checked={on} sx={{ ml: 1 }} />
    </div>
  );
}
