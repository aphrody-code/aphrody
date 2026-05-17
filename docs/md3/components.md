# Material Web Components (`@material/web`)

L'implémentation de MD3 dans notre projet repose sur la bibliothèque officielle `@material/web` de Google. 
Ces composants sont des Custom Elements HTML natifs. Ils fonctionnent avec Vanilla JS, Lit, React, Angular, Vue, ou tout autre framework web.

## Installation et Importation

Dans votre projet (`packages/ui`), assurez-vous d'importer les composants dont vous avez besoin. L'importation déclare automatiquement le Custom Element dans le registre du navigateur.

```javascript
// Exemple d'importation dans un fichier d'entrée (e.g. index.js ou main.ts)
import '@material/web/button/filled-button.js';
import '@material/web/button/outlined-button.js';
import '@material/web/checkbox/checkbox.js';
import '@material/web/textfield/filled-text-field.js';
```

## Utilisation en HTML

Une fois importés, les composants s'utilisent comme des balises HTML standards, préfixées par `md-`.

### Boutons

```html
<md-filled-button>Action Principale</md-filled-button>
<md-outlined-button>Action Secondaire</md-outlined-button>
<md-text-button>Annuler</md-text-button>
<md-elevated-button>Sauvegarder</md-elevated-button>
```

### Champs de texte

```html
<md-filled-text-field label="Nom d'utilisateur" type="text"></md-filled-text-field>
<md-outlined-text-field label="Mot de passe" type="password"></md-outlined-text-field>
```

### Composants de sélection

```html
<label>
  <md-checkbox checked></md-checkbox>
  Activer l'option
</label>

<md-radio name="theme" value="dark"></md-radio>
<md-radio name="theme" value="light" checked></md-radio>
```

## Gestion des Événements et Propriétés

Puisqu'ils sont natifs, l'interaction se fait via l'API DOM standard :

```javascript
const button = document.querySelector('md-filled-button');

// Écouter un événement
button.addEventListener('click', () => {
  console.log('Bouton cliqué !');
});

// Modifier une propriété
button.disabled = true;

const textField = document.querySelector('md-filled-text-field');
textField.addEventListener('input', (e) => {
  console.log('Valeur:', e.target.value);
});
```

## Intégration A2UI (Agent-to-User Interface)

Ces composants sont parfaits pour le système `A2UI` cloné dans `packages/a2ui`. Les agents peuvent générer des schémas JSON qui se traduisent directement par ces balises `<md-*>` natives, garantissant un rendu sécurisé et universel sans exécution de code JavaScript dangereux.
