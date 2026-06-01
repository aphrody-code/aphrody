// Header model picker — an M3 menu listing every available model with a check on
// the active one. Mirrors open-webui's ModelSelector (single-model variant).

import { MdIcon, MdMenuItem, MdTextButton } from "@aphrody-code/m3-react";
import { Menu } from "../ui/Menu.tsx";
import { useModels } from "../../api/queries.ts";

export function ModelSelector({
  value,
  onChange,
}: {
  value: string;
  onChange: (id: string) => void;
}) {
  const { data: models = [] } = useModels();
  const active = models.find((m) => m.id === value);

  return (
    <Menu
      trigger={({ toggle }) => (
        <MdTextButton onClick={toggle} trailingIcon>
          {active?.name ?? value ?? "Select model"}
          <MdIcon slot="icon">expand_more</MdIcon>
        </MdTextButton>
      )}
    >
      {models.map((m) => (
        <MdMenuItem key={m.id} onClick={() => onChange(m.id)}>
          {m.id === value && <MdIcon slot="start">check</MdIcon>}
          <span slot="headline">{m.name}</span>
          <span slot="supporting-text">{m.description ?? m.owned_by}</span>
        </MdMenuItem>
      ))}
    </Menu>
  );
}
