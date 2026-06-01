import { Button } from "@mui/material";
import SaveIcon from "@mui/icons-material/Save";

export function Demo() {
  return (
    <div>
      <Button variant="contained" onClick={() => {}}>
        Sauver
      </Button>
      <Button variant="outlined" startIcon={<SaveIcon />}>
        Avec icône
      </Button>
      <Button variant="text" disabled>
        Texte
      </Button>
      <Button sx={{ mt: 2 }}>Défaut</Button>
    </div>
  );
}
