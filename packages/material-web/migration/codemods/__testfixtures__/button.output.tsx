import "@aphrody/material-web/icon/icon.js";
import { MdFilledButton, MdOutlinedButton, MdTextButton } from "@aphrody/m3-react";
import SaveIcon from "@mui/icons-material/Save";

export function Demo() {
  return (
    <div>
      <MdFilledButton onClick={() => {}}>Sauver</MdFilledButton>
      <MdOutlinedButton>
        <md-icon slot="icon">{<SaveIcon />}</md-icon>Avec icône
      </MdOutlinedButton>
      <MdTextButton disabled>Texte</MdTextButton>
      {/* MIGRATION-TODO: prop `sx` retiree : convertir en classes Tailwind (host/layout) + tokens --md-sys-* (le shadow DOM n-est pas atteignable par Tailwind, cf. §6). */}
      <MdFilledButton>Défaut</MdFilledButton>
    </div>
  );
}
