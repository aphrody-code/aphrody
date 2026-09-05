# Plateforme coordonnée des sites Aphrody

## Objectif

Les sites `aphrody.com`, `bxc.aphrody.com`, `nie.aphrody.com` et
`n2b.aphrody.com` forment une même plateforme éditoriale sans devenir un
monolithe. Chaque produit garde son dépôt, son binaire et son cycle de
déploiement. Les contrats visuels, médias, sécurité et observabilité sont
communs et versionnés depuis Aphrody.

## Répartition des services

| Site | Dépôt | Port local | Rôle |
|---|---|---:|---|
| Aphrody | dépôt `aphrody` | 8083 | hôte, APIs, téléchargements et socle commun |
| BXC | dépôt `bxc` | 8084 | vitrine de l'automatisation navigateur |
| Niers | dépôt `niers` | 8085 | vitrine Inazuma et contenus autorisés |
| N2B | dépôt `n2b` | 8086 | documentation et téléchargement du CLI |

Tous les services écoutent sur `127.0.0.1`. nginx termine TLS, applique les
limites et route selon le nom d'hôte. Aucun serveur applicatif n'est exposé
directement.

## Socle partagé

Le socle cible est une crate Rust `aphrody-web` dans le workspace Aphrody,
consommée avec une révision Git épinglée par les trois autres dépôts. Elle ne
contient aucune logique métier. Son API publique se limite à :

- les tokens de thème, typographie, espacements et composants accessibles ;
- le squelette HTML, les métadonnées, `security.txt` et `robots.txt` ;
- les en-têtes CSP/HSTS et les réponses d'erreur cohérentes ;
- un manifeste média typé et le calcul d'URL versionnées ;
- les endpoints communs `/healthz`, `/version` et `/downloads`.

Les médias canoniques vivent sous `media/<produit>/<identifiant>/<version>/`.
Leur manifeste contient le hash BLAKE3, le type MIME, les dimensions, la licence
et l'attribution éventuelle. Le contenu public est servi par `cdn.aphrody.com`
avec URLs immuables; les dépôts ne dupliquent pas les originaux.

## Coordination et livraison

Chaque site produit un binaire autonome, une image OCI optionnelle et un
manifeste de version. Une matrice CI teste chaque dépôt contre la version
minimale et courante d'`aphrody-web`. Les mises à jour incompatibles passent par
une nouvelle version majeure et une PR coordonnée dans chaque dépôt.

Ordre recommandé : extraire `aphrody-web` du serveur actuel, migrer Aphrody,
créer ensuite les vitrines N2B, BXC et Niers, puis activer leurs vhosts. Jusqu'à
la présence de leur binaire, les sous-domaines utilisent volontairement la page
blanche saine d'Aphrody.
