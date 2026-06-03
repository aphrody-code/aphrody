// =============================================================================
// AVANT — Écran "Paramètres de compte" (MUI / @mui/material@9.0.1, Material 2)
// =============================================================================
// Écran réaliste et non-trivial : barre d'app, onglets, formulaire varié
// (TextField, Select, Checkbox, Switch, Radio), carte récapitulative, liste de
// sessions, dialog de confirmation, snackbar de feedback, alerte d'erreur.
// Layout 100 % MUI : Box / Stack / Grid + sx. Styling Emotion via le thème.
//
// C'est ce fichier que `../after/AccountSettings.tsx` migre composant par
// composant. La table de correspondance détaillée est dans ../MIGRATION-NOTES.md.

import * as React from "react";
import { ThemeProvider, CssBaseline } from "@mui/material";
import AppBar from "@mui/material/AppBar";
import Toolbar from "@mui/material/Toolbar";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Grid from "@mui/material/Grid";
import Card from "@mui/material/Card";
import CardHeader from "@mui/material/CardHeader";
import CardContent from "@mui/material/CardContent";
import CardActions from "@mui/material/CardActions";
import Tabs from "@mui/material/Tabs";
import Tab from "@mui/material/Tab";
import TextField from "@mui/material/TextField";
import MenuItem from "@mui/material/MenuItem";
import FormControl from "@mui/material/FormControl";
import InputLabel from "@mui/material/InputLabel";
import Select from "@mui/material/Select";
import FormControlLabel from "@mui/material/FormControlLabel";
import Checkbox from "@mui/material/Checkbox";
import Switch from "@mui/material/Switch";
import Radio from "@mui/material/Radio";
import RadioGroup from "@mui/material/RadioGroup";
import FormLabel from "@mui/material/FormLabel";
import Button from "@mui/material/Button";
import IconButton from "@mui/material/IconButton";
import Typography from "@mui/material/Typography";
import Divider from "@mui/material/Divider";
import List from "@mui/material/List";
import ListItem from "@mui/material/ListItem";
import ListItemText from "@mui/material/ListItemText";
import ListItemIcon from "@mui/material/ListItemIcon";
import Dialog from "@mui/material/Dialog";
import DialogTitle from "@mui/material/DialogTitle";
import DialogContent from "@mui/material/DialogContent";
import DialogContentText from "@mui/material/DialogContentText";
import DialogActions from "@mui/material/DialogActions";
import Snackbar from "@mui/material/Snackbar";
import Alert from "@mui/material/Alert";
import MenuIcon from "@mui/icons-material/Menu";
import DevicesIcon from "@mui/icons-material/Devices";

import { makeTheme } from "./theme";

export default function AccountSettings() {
  // --- État du thème (mode clair/sombre) ------------------------------------
  const [mode, setMode] = React.useState<"light" | "dark">("light");
  const theme = React.useMemo(() => makeTheme(mode), [mode]);

  // --- État des onglets -----------------------------------------------------
  const [tab, setTab] = React.useState(0);

  // --- État du formulaire (controlled inputs) -------------------------------
  const [displayName, setDisplayName] = React.useState("Alex");
  const [email, setEmail] = React.useState("user@example.com");
  const [language, setLanguage] = React.useState("fr");
  const [newsletter, setNewsletter] = React.useState(true);
  const [twoFactor, setTwoFactor] = React.useState(false);
  const [visibility, setVisibility] = React.useState("public");

  // --- Validation ------------------------------------------------------------
  const emailError = email.length > 0 && !email.includes("@");

  // --- Dialog + Snackbar -----------------------------------------------------
  const [confirmOpen, setConfirmOpen] = React.useState(false);
  const [snackOpen, setSnackOpen] = React.useState(false);

  const handleSave = () => {
    if (emailError) return;
    setSnackOpen(true);
  };

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />

      {/* ----- AppBar + Toolbar ----- */}
      <AppBar position="static" color="primary">
        <Toolbar>
          <IconButton edge="start" color="inherit" aria-label="menu" sx={{ mr: 2 }}>
            <MenuIcon />
          </IconButton>
          <Typography variant="h6" sx={{ flexGrow: 1 }}>
            Paramètres de compte
          </Typography>
          <Switch
            checked={mode === "dark"}
            onChange={(e) => setMode(e.target.checked ? "dark" : "light")}
            inputProps={{ "aria-label": "Activer le thème sombre" }}
          />
        </Toolbar>
      </AppBar>

      {/* ----- Conteneur principal (Box + sx) ----- */}
      <Box sx={{ maxWidth: 880, mx: "auto", p: 3 }}>
        {/* ----- Onglets ----- */}
        <Tabs
          value={tab}
          onChange={(_e, v) => setTab(v)}
          sx={{ mb: 3 }}
          aria-label="sections du compte"
        >
          <Tab label="Profil" />
          <Tab label="Sécurité" />
          <Tab label="Sessions" />
        </Tabs>

        {/* ===== Onglet 0 : Profil ===== */}
        {tab === 0 && (
          <Card variant="outlined">
            <CardHeader
              title="Profil public"
              subheader="Ces informations sont visibles par les autres membres."
            />
            <CardContent>
              <Grid container spacing={2}>
                <Grid item xs={12} sm={6}>
                  <TextField
                    fullWidth
                    label="Nom affiché"
                    value={displayName}
                    onChange={(e) => setDisplayName(e.target.value)}
                  />
                </Grid>
                <Grid item xs={12} sm={6}>
                  <TextField
                    fullWidth
                    type="email"
                    label="E-mail"
                    value={email}
                    error={emailError}
                    helperText={emailError ? "Adresse e-mail invalide" : " "}
                    onChange={(e) => setEmail(e.target.value)}
                  />
                </Grid>
                <Grid item xs={12} sm={6}>
                  <FormControl fullWidth>
                    <InputLabel id="lang-label">Langue</InputLabel>
                    <Select
                      labelId="lang-label"
                      label="Langue"
                      value={language}
                      onChange={(e) => setLanguage(e.target.value)}
                    >
                      <MenuItem value="fr">Français</MenuItem>
                      <MenuItem value="en">English</MenuItem>
                      <MenuItem value="es">Español</MenuItem>
                    </Select>
                  </FormControl>
                </Grid>
                <Grid item xs={12}>
                  <FormControl>
                    <FormLabel id="visibility-label">Visibilité du profil</FormLabel>
                    <RadioGroup
                      row
                      aria-labelledby="visibility-label"
                      value={visibility}
                      onChange={(e) => setVisibility(e.target.value)}
                    >
                      <FormControlLabel value="public" control={<Radio />} label="Public" />
                      <FormControlLabel value="private" control={<Radio />} label="Privé" />
                    </RadioGroup>
                  </FormControl>
                </Grid>
                <Grid item xs={12}>
                  <FormControlLabel
                    control={
                      <Checkbox
                        checked={newsletter}
                        onChange={(e) => setNewsletter(e.target.checked)}
                      />
                    }
                    label="Recevoir la newsletter mensuelle"
                  />
                </Grid>
              </Grid>
            </CardContent>
            <CardActions sx={{ justifyContent: "flex-end" }}>
              <Button variant="text" onClick={() => window.history.back()}>
                Annuler
              </Button>
              <Button variant="contained" onClick={handleSave}>
                Enregistrer
              </Button>
            </CardActions>
          </Card>
        )}

        {/* ===== Onglet 1 : Sécurité ===== */}
        {tab === 1 && (
          <Card variant="outlined">
            <CardHeader title="Sécurité" />
            <CardContent>
              <Stack spacing={2}>
                {/* Alerte d'avertissement — composant MUI sans équivalent md (gap). */}
                <Alert severity="warning">
                  L'authentification à deux facteurs n'est pas activée.
                </Alert>
                <FormControlLabel
                  control={
                    <Switch checked={twoFactor} onChange={(e) => setTwoFactor(e.target.checked)} />
                  }
                  label="Activer l'authentification à deux facteurs"
                />
                <Divider />
                <Button variant="outlined" color="error" onClick={() => setConfirmOpen(true)}>
                  Supprimer mon compte
                </Button>
              </Stack>
            </CardContent>
          </Card>
        )}

        {/* ===== Onglet 2 : Sessions ===== */}
        {tab === 2 && (
          <Card variant="outlined">
            <CardHeader title="Sessions actives" />
            <CardContent sx={{ p: 0 }}>
              <List>
                {[
                  { device: "MacBook Pro — Paris", current: true },
                  { device: "iPhone 15 — Lyon", current: false },
                ].map((s) => (
                  <ListItem key={s.device}>
                    <ListItemIcon>
                      <DevicesIcon />
                    </ListItemIcon>
                    <ListItemText
                      primary={s.device}
                      secondary={s.current ? "Session actuelle" : "Dernière activité il y a 2 j"}
                    />
                  </ListItem>
                ))}
              </List>
            </CardContent>
          </Card>
        )}
      </Box>

      {/* ----- Dialog de confirmation ----- */}
      <Dialog open={confirmOpen} onClose={() => setConfirmOpen(false)}>
        <DialogTitle>Supprimer le compte ?</DialogTitle>
        <DialogContent>
          <DialogContentText>
            Cette action est irréversible. Toutes vos données seront effacées.
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirmOpen(false)}>Annuler</Button>
          <Button color="error" onClick={() => setConfirmOpen(false)}>
            Supprimer
          </Button>
        </DialogActions>
      </Dialog>

      {/* ----- Snackbar de feedback ----- */}
      <Snackbar
        open={snackOpen}
        autoHideDuration={4000}
        onClose={() => setSnackOpen(false)}
        message="Modifications enregistrées"
      />
    </ThemeProvider>
  );
}
