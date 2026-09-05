import { MdDialog, MdFilledButton, MdTextButton } from "@aphrody/m3-react";

export function Confirm({ open }) {
  return (
    <MdDialog open={open}>
      {/* MIGRATION-TODO: DialogTitle -> <div slot="headline"> dans <MdDialog> : verifier l-imbrication (slot direct). */}
      <div slot="headline">Confirmer</div>
      {/* MIGRATION-TODO: DialogContent -> <div slot="content"> dans <MdDialog> : verifier l-imbrication (slot direct). */}
      <div slot="content">Es-tu sûr ?</div>
      {/* MIGRATION-TODO: DialogActions -> <div slot="actions"> dans <MdDialog> : verifier l-imbrication (slot direct). */}
      <div slot="actions">
        <MdTextButton>Annuler</MdTextButton>
        <MdFilledButton>OK</MdFilledButton>
      </div>
    </MdDialog>
  );
}
