<!-- SPDX-License-Identifier: Apache-2.0 -->
# API publique aphrody.com

La future API publique doit être ajoutée au binaire Rust `aphrody-site` avec
Axum 0.8 et Tokio. Aucun runtime JavaScript, panneau d'administration ou état
utilisateur n'est exposé par défaut.

## Contrat minimal

- Les routes publiques versionnées utilisent `/api/v1/*`.
- `/healthz` reste une sonde sans état et sans donnée d'infrastructure.
- Les téléchargements versionnés utilisent `/downloads/<version>/<artifact>`.
- Toute réponse d'erreur utilise un statut HTTP précis et ne révèle ni chemin
  local, ni secret, ni identité personnelle.
- Les journaux d'accès restent désactivés tant qu'une politique de rétention et
  d'anonymisation n'est pas explicitement adoptée.
- A2A et MCP restent privés ; ils ne sont jamais ajoutés à cette origine par
  simple proxy nginx.

Voir [`DOMAIN.md`](DOMAIN.md) pour le déploiement.
