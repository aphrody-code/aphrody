# Theming & Design Tokens (Material You)

L'un des plus grands apports de Material Design 3 est son système de Design Tokens basé sur le **Dynamic Color** et l'espace colorimétrique **HCT (Hue, Chroma, Tone)**.

## Design Tokens (CSS Custom Properties)

Les composants `@material/web` n'utilisent pas de classes CSS utilitaires complexes ou de préprocesseurs pour le thème global. Ils reposent entièrement sur les **CSS Custom Properties** (variables CSS).

Pour thémer l'application, vous modifiez les variables globales (préfixées par `--md-sys-color-`) dans votre feuille de style.

```css
:root {
  /* Couleurs primaires */
  --md-sys-color-primary: #006494;
  --md-sys-color-on-primary: #ffffff;
  --md-sys-color-primary-container: #cae6ff;
  --md-sys-color-on-primary-container: #001e30;

  /* Arrière-plans et surfaces */
  --md-sys-color-background: #fdfcff;
  --md-sys-color-on-background: #1a1c1e;
  --md-sys-color-surface: #fdfcff;
  --md-sys-color-on-surface: #1a1c1e;

  /* Erreurs */
  --md-sys-color-error: #ba1a1a;
  --md-sys-color-on-error: #ffffff;
}

@media (prefers-color-scheme: dark) {
  :root {
    --md-sys-color-primary: #8dcdff;
    --md-sys-color-on-primary: #003450;
    --md-sys-color-primary-container: #004b71;
    --md-sys-color-on-primary-container: #cae6ff;
    
    --md-sys-color-background: #1a1c1e;
    --md-sys-color-on-background: #e2e2e5;
  }
}
```

## Personnalisation locale

Vous pouvez styliser un composant spécifique en surchargeant ses tokens locaux (préfixés selon le composant).

```css
md-filled-button.danger-btn {
  --md-filled-button-container-color: var(--md-sys-color-error);
  --md-filled-button-label-text-color: var(--md-sys-color-on-error);
  --md-filled-button-hover-container-color: #ff0000;
}
```

## Élévation (Elevation) et Surface Containers

Dans les versions récentes de MD3, le système d'élévation basé sur la teinte (Elevation 1 à 5) a évolué vers le concept explicite de **Surface Containers**. Au lieu d'assombrir/éclaircir dynamiquement la surface avec la couleur primaire selon son niveau d'élévation, on utilise des variables dédiées pour les conteneurs :

*   `--md-sys-color-surface-container-lowest`
*   `--md-sys-color-surface-container-low`
*   `--md-sys-color-surface-container`
*   `--md-sys-color-surface-container-high`
*   `--md-sys-color-surface-container-highest`

Pour gérer **l'ombre portée visuelle**, la bibliothèque `@material/web` fournit désormais un composant dédié `<md-elevation>` que vous pouvez inclure dans la structure DOM de vos éléments personnalisés, ou utiliser conjointement avec ces variables de surface.
