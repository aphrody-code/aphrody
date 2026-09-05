import { Dialog, DialogTitle, DialogContent, DialogActions, Button } from "@mui/material";

export function Confirm({ open }) {
  return (
    <Dialog open={open}>
      <DialogTitle>Confirmer</DialogTitle>
      <DialogContent>Es-tu sûr ?</DialogContent>
      <DialogActions>
        <Button variant="text">Annuler</Button>
        <Button variant="contained">OK</Button>
      </DialogActions>
    </Dialog>
  );
}
