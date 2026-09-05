<!-- SPDX-License-Identifier: Apache-2.0 -->
# État d'aphrody.com

## Objectif

`aphrody.com` est une origine technique minimale destinée aux futures API,
pages de téléchargement et services publics d'Aphrody. La racine est une page
blanche. Aucun compte utilisateur, panneau d'administration, cookie, traceur
ou journal d'accès visiteur n'est actif.

## Architecture active

- Origine : binaire Rust `aphrody-site`, Axum 0.8 sur Tokio 1.52.
- Écoute privée : `127.0.0.1:8083`.
- Exposition : nginx sur HTTP/HTTPS ; HTTP redirige vers HTTPS.
- TLS : certificat Let's Encrypt avec renouvellement automatique.
- Service : `aphrody-site.service`, activé au démarrage.
- Sécurité : CSP restrictive, protection anti-framing, MIME sniffing désactivé,
  aucune journalisation nginx pour ces hôtes.

## Noms DNS

| Nom | Cible | État |
|---|---|---|
| `aphrody.com` | `51.77.147.152` | Origine principale, page blanche |
| `www.aphrody.com` | `aphrody.com` | Alias réservé |
| `api.aphrody.com` | `aphrody.com` | Futures API versionnées `/api/v1/*` |
| `downloads.aphrody.com` | `aphrody.com` | Futurs artefacts versionnés |
| `mcp.aphrody.com` | `127.0.0.1:8808/mcp` via nginx | MCP public, Bearer lecture/admin obligatoire |
| `cdn.aphrody.com` | `aphrody.com` | Réservé aux futurs fichiers publics |
| `bot.aphrody.com` | `aphrody.com` | Réservé ; aucun bot privé exposé |
| `admin.aphrody.com` | `aphrody.com` | Réservé ; aucune administration exposée |
| `bxc.aphrody.com` | `aphrody.com` | Réservé à la future vitrine Rust de BXC |
| `nie.aphrody.com` | `aphrody.com` | Réservé à la future vitrine Rust de Niers |

Tous les alias servent actuellement la même origine blanche et retournent 404
pour les routes non définies.

## Messagerie

- Adresse prévue : `contact@aphrody.com`.
- MX OVH et SPF sont conservés. Aucun enregistrement DKIM ou autodiscover
  public n'a été observé lors du dernier audit.
- DMARC est actif en mode observation stricte.
- La création effective de la boîte reste bloquée tant que la clé API OVH ne
  possède pas les autorisations Email Domain, MX Plan ou Email Pro.
- Configuration et procédure : [`MAIL-STACK.md`](MAIL-STACK.md).

## Fichiers de référence

- Serveur : [`../crates/aphrody-site/src/main.rs`](../crates/aphrody-site/src/main.rs)
- nginx : [`../deploy/nginx/aphrody.com.conf`](../deploy/nginx/aphrody.com.conf)
- systemd : [`../deploy/systemd/aphrody-site.service`](../deploy/systemd/aphrody-site.service)
- Déploiement : [`DOMAIN.md`](DOMAIN.md)
- Contrat des futures API : [`api-unified-pattern.md`](api-unified-pattern.md)
