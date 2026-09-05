// =============================================================================
// APRÈS — Écran "Paramètres de compte" migré vers @aphrody/m3-react
// =============================================================================
// Même UI que ../before/AccountSettings.tsx, migrée selon le contrat partagé :
//   • Wrappers @lit/react des éléments <md-*> réels   (contrat §2, §3)
//   • Layout (ex-Box/Stack/Grid) → utilitaires Tailwind sur des <div> (§3, §6)
//   • Thème → tokens --md-sys-* importés via ./theme.css                (§5)
//   • Events controlled adaptés : e.target.value, plus de (e, value)    (§4)
//   • MdDialog ouvert impérativement via ref.show() (pas de prop `open`) (§3)
//   • Slots md (headline/content/actions, leading/trailing, start/end)  (§4)
//
// La table de correspondance point par point est dans ../MIGRATION-NOTES.md.
// Tous les noms d'éléments/props/slots sont vérifiés dans material-web/ — voir
// les renvois `material-web/...:ligne` dans MIGRATION-NOTES.md.

import * as React from "react";
import "./theme.css"; // tokens --md-sys-* (remplace ThemeProvider/createTheme)

// --- Wrappers @aphrody/m3-react (réexportés depuis migration/wrappers) -------
// (cf. ../../wrappers/index.ts ; convention de nommage contrat §2)
import {
  MdTopAppBar,
  MdIconButton,
  MdIcon,
  MdSwitch,
  MdTabs,
  MdPrimaryTab,
  MdOutlinedCard,
  MdOutlinedTextField,
  MdFilledSelect,
  MdSelectOption,
  MdCheckbox,
  MdRadio,
  MdDivider,
  MdList,
  MdListItem,
  MdTextButton,
  MdFilledButton,
  MdOutlinedButton,
  MdDialog,
  MdSnackbar,
} from "../../wrappers";

// Shim pour Alert (gap MUI → md, cf. 05-gap-analysis.md). Rendu = surface
// tokenisée + md-icon ; pas d'élément md-alert (n'existe pas — ne pas inventer,
// contrat §7.2).
import { M3Alert } from "./shims/M3Alert";

export default function AccountSettings() {
  // --- Thème : bascule clair/sombre par classe sur <html> --------------------
  // Remplace le re-render de createTheme : on ne fait que toggler .dark (§5).
  const [dark, setDark] = React.useState(false);
  React.useEffect(() => {
    document.documentElement.classList.toggle("dark", dark);
  }, [dark]);

  // --- État des onglets (inchangé côté React) --------------------------------
  const [tab, setTab] = React.useState(0);

  // --- État du formulaire (controlled inputs) --------------------------------
  const [displayName, setDisplayName] = React.useState("Alex");
  const [email, setEmail] = React.useState("user@example.com");
  const [language, setLanguage] = React.useState("fr");
  const [newsletter, setNewsletter] = React.useState(true);
  const [twoFactor, setTwoFactor] = React.useState(false);
  const [visibility, setVisibility] = React.useState("public");

  const emailError = email.length > 0 && !email.includes("@");

  // --- Dialog : ref impérative (md-dialog s'ouvre via .show()/.close()) ------
  // PIÈGE (§3) : md-dialog a bien une prop `open`, mais le pattern recommandé
  // est l'API impérative show()/close() qui pilote le top-layer. On garde la
  // ref typée sur l'élément natif.
  const dialogRef = React.useRef<HTMLElementTagNameMap["md-dialog"]>(null);

  // --- Snackbar : ref impérative également (.show()/.close()) -----------------
  const snackRef = React.useRef<HTMLElementTagNameMap["md-snackbar"]>(null);

  const handleSave = () => {
    if (emailError) return;
    snackRef.current?.show();
  };

  return (
    <>
      {/* ----- Top app bar (ex AppBar + Toolbar) -----
       * slots vérifiés (material-web/appbar/internal/top-app-bar.ts:24-26) :
       * leading | (défaut = titre) | trailing. variant="small" = 1 rangée 64dp. */}
      <MdTopAppBar variant="small">
        <MdIconButton slot="leading" aria-label="menu">
          <MdIcon>menu</MdIcon>
        </MdIconButton>
        {/* Titre : texte direct dans le slot par défaut (ex Typography h6). */}
        <span>Paramètres de compte</span>
        {/* Switch de thème — md-switch utilise `selected` (PAS `checked`),
         * event `change` → e.target.selected (contrat §4). */}
        <MdSwitch
          slot="trailing"
          selected={dark}
          aria-label="Activer le thème sombre"
          onChange={(e) => setDark((e.target as HTMLInputElement & { selected: boolean }).selected)}
        />
      </MdTopAppBar>

      {/* ----- Conteneur principal -----
       * ex <Box sx={{maxWidth:880, mx:'auto', p:3}}> → utilitaires Tailwind sur
       * un <div> hôte (le sx n'a pas d'équivalent, contrat §4 + §6). */}
      <div className="mx-auto max-w-[880px] p-6">
        {/* ----- Onglets (ex Tabs/Tab) -----
         * md-tabs expose activeTabIndex + event `change`
         * (material-web/tabs/internal/tabs.ts:15-16). On lit la sélection sur
         * la cible : e.target.activeTabIndex. */}
        <MdTabs
          className="mb-6"
          activeTabIndex={tab}
          aria-label="sections du compte"
          onChange={(e) => setTab((e.target as HTMLElementTagNameMap["md-tabs"]).activeTabIndex)}
        >
          <MdPrimaryTab>Profil</MdPrimaryTab>
          <MdPrimaryTab>Sécurité</MdPrimaryTab>
          <MdPrimaryTab>Sessions</MdPrimaryTab>
        </MdTabs>

        {/* ===== Onglet 0 : Profil ===== */}
        {tab === 0 && (
          // ex <Card variant="outlined"> → md-outlined-card (fork labs, slot
          // unique par défaut). CardHeader/Content/Actions → contenu + layout
          // Tailwind, car md-card n'a pas de sous-slots (labs/card/internal/card.ts:19).
          <MdOutlinedCard className="block p-4">
            {/* ex CardHeader : titre + sous-titre via md-type/typescale */}
            <header className="mb-4">
              <h2 className="m-0 text-[length:var(--md-sys-typescale-title-large-size,1.375rem)]">
                Profil public
              </h2>
              <p className="m-0 text-[color:var(--md-sys-color-on-surface-variant)]">
                Ces informations sont visibles par les autres membres.
              </p>
            </header>

            {/* ex <Grid container spacing={2}> → grille Tailwind (contrat §6) */}
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              {/* TextField "outlined" : md-outlined-text-field
               * (props label/value/error/errorText vérifiées
               * material-web/textfield/internal/text-field.ts:109-180).
               * Controlled : onInput → e.target.value (contrat §4). */}
              <MdOutlinedTextField
                label="Nom affiché"
                value={displayName}
                onInput={(e) => setDisplayName((e.target as HTMLInputElement).value)}
              />
              <MdOutlinedTextField
                type="email"
                label="E-mail"
                value={email}
                error={emailError}
                // helperText → supportingText/error-text (§4)
                errorText={emailError ? "Adresse e-mail invalide" : undefined}
                onInput={(e) => setEmail((e.target as HTMLInputElement).value)}
              />

              {/* ex FormControl+InputLabel+Select → md-filled-select.
               * Le label est une prop de l'élément (pas un InputLabel séparé).
               * MenuItem → md-select-option (value = prop, libellé = slot
               * headline). Event `change` → e.target.value
               * (material-web/select/internal/select.ts:62-105, 168-180). */}
              <MdFilledSelect
                label="Langue"
                value={language}
                onChange={(e) =>
                  setLanguage((e.target as HTMLElementTagNameMap["md-filled-select"]).value)
                }
              >
                <MdSelectOption value="fr">
                  <div slot="headline">Français</div>
                </MdSelectOption>
                <MdSelectOption value="en">
                  <div slot="headline">English</div>
                </MdSelectOption>
                <MdSelectOption value="es">
                  <div slot="headline">Español</div>
                </MdSelectOption>
              </MdFilledSelect>

              {/* ex RadioGroup/Radio → groupe de md-radio liés par `name`
               * (material-web/radio/internal/radio.ts : checked/name/value).
               * Pas de FormControlLabel : on compose le <label> nous-mêmes. */}
              <fieldset className="col-span-full m-0 border-0 p-0">
                <legend className="mb-2 text-[color:var(--md-sys-color-on-surface-variant)]">
                  Visibilité du profil
                </legend>
                <div className="flex gap-6">
                  <label className="flex items-center gap-2">
                    <MdRadio
                      name="visibility"
                      value="public"
                      checked={visibility === "public"}
                      onChange={(e) => setVisibility((e.target as HTMLInputElement).value)}
                    />
                    Public
                  </label>
                  <label className="flex items-center gap-2">
                    <MdRadio
                      name="visibility"
                      value="private"
                      checked={visibility === "private"}
                      onChange={(e) => setVisibility((e.target as HTMLInputElement).value)}
                    />
                    Privé
                  </label>
                </div>
              </fieldset>

              {/* ex FormControlLabel+Checkbox → md-checkbox dans un <label>.
               * md-checkbox utilise `checked` (material-web/checkbox/internal/
               * checkbox.ts:62) + event `change` → e.target.checked (§4). */}
              <label className="col-span-full flex items-center gap-2">
                <MdCheckbox
                  checked={newsletter}
                  onChange={(e) => setNewsletter((e.target as HTMLInputElement).checked)}
                />
                Recevoir la newsletter mensuelle
              </label>
            </div>

            {/* ex CardActions justify-end → div Tailwind. Boutons : Button
             * variant="text"→md-text-button, "contained"→md-filled-button (§3). */}
            <div className="mt-4 flex justify-end gap-2">
              <MdTextButton onClick={() => window.history.back()}>Annuler</MdTextButton>
              <MdFilledButton onClick={handleSave}>Enregistrer</MdFilledButton>
            </div>
          </MdOutlinedCard>
        )}

        {/* ===== Onglet 1 : Sécurité ===== */}
        {tab === 1 && (
          <MdOutlinedCard className="block p-4">
            <header className="mb-4">
              <h2 className="m-0 text-[length:var(--md-sys-typescale-title-large-size,1.375rem)]">
                Sécurité
              </h2>
            </header>

            {/* ex <Stack spacing={2}> → flex-col + gap Tailwind (contrat §6) */}
            <div className="flex flex-col gap-4">
              {/* GAP : MUI <Alert severity="warning"> n'a pas d'équivalent md.
               * Shim tokenisé (cf. 05-gap-analysis.md). TODO: remplacer par
               * un éventuel md-banner si le fork l'ajoute. */}
              <M3Alert severity="warning">
                L'authentification à deux facteurs n'est pas activée.
              </M3Alert>

              <label className="flex items-center gap-3">
                <MdSwitch
                  selected={twoFactor}
                  onChange={(e) =>
                    setTwoFactor((e.target as HTMLInputElement & { selected: boolean }).selected)
                  }
                />
                Activer l'authentification à deux facteurs
              </label>

              <MdDivider />

              {/* Button outlined + color="error" → md-outlined-button.
               * La couleur d'erreur passe par un token local (pas de prop
               * `color` côté md ; on surcharge --md-sys-color-primary sur le
               * host via style inline, contrat §4 : sx supprimé → style). */}
              <MdOutlinedButton
                className="self-start"
                style={{
                  ["--md-sys-color-primary" as string]: "var(--md-sys-color-error)",
                  ["--md-sys-color-outline" as string]: "var(--md-sys-color-error)",
                }}
                onClick={() => dialogRef.current?.show()}
              >
                Supprimer mon compte
              </MdOutlinedButton>
            </div>
          </MdOutlinedCard>
        )}

        {/* ===== Onglet 2 : Sessions ===== */}
        {tab === 2 && (
          <MdOutlinedCard className="block p-4">
            <header className="mb-2">
              <h2 className="m-0 text-[length:var(--md-sys-typescale-title-large-size,1.375rem)]">
                Sessions actives
              </h2>
            </header>

            {/* ex List/ListItem/ListItemText/ListItemIcon → md-list/md-list-item.
             * Slots vérifiés (material-web/list/internal/listitem/list-item.ts:
             * 90-91, 184-186) : start | headline | supporting-text | end. */}
            <MdList>
              {[
                { device: "MacBook Pro — Paris", current: true },
                { device: "iPhone 15 — Lyon", current: false },
              ].map((s) => (
                <MdListItem key={s.device}>
                  <MdIcon slot="start">devices</MdIcon>
                  <div slot="headline">{s.device}</div>
                  <div slot="supporting-text">
                    {s.current ? "Session actuelle" : "Dernière activité il y a 2 j"}
                  </div>
                </MdListItem>
              ))}
            </MdList>
          </MdOutlinedCard>
        )}
      </div>

      {/* ----- Dialog de confirmation (ex Dialog/DialogTitle/Content/Actions) -----
       * Slots vérifiés (material-web/dialog/internal/dialog.ts:307-325) :
       * icon | headline | content | actions. Ouverture via dialogRef.show(). */}
      <MdDialog ref={dialogRef}>
        <div slot="headline">Supprimer le compte ?</div>
        <form slot="content" id="delete-form" method="dialog">
          Cette action est irréversible. Toutes vos données seront effacées.
        </form>
        <div slot="actions">
          {/* `form` + value : md-dialog ferme et expose returnValue (§3) */}
          <MdTextButton onClick={() => dialogRef.current?.close()}>Annuler</MdTextButton>
          <MdTextButton
            style={{
              ["--md-sys-color-primary" as string]: "var(--md-sys-color-error)",
            }}
            onClick={() => dialogRef.current?.close("delete")}
          >
            Supprimer
          </MdTextButton>
        </div>
      </MdDialog>

      {/* ----- Snackbar (ex Snackbar message=…) -----
       * md-snackbar (fork) : labelText/timeout-ms props, show()/close()
       * (material-web/snackbar/internal/snackbar.ts:43-65). autoHideDuration
       * → timeout-ms. */}
      <MdSnackbar ref={snackRef} labelText="Modifications enregistrées" timeout-ms={4000} />
    </>
  );
}
