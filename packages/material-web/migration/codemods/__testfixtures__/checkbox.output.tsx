import { MdCheckbox, MdSwitch } from "@aphrody-code/m3-react";

export function Toggles() {
  return (
    <div>
      <MdCheckbox checked={ok} onChange={(e) => setOk(e.target.checked)} />
      {/* MIGRATION-TODO: prop `sx` retiree : convertir en classes Tailwind (host/layout) + tokens --md-sys-* (le shadow DOM n-est pas atteignable par Tailwind, cf. §6). */}
      <MdSwitch checked={on} />
    </div>
  );
}
