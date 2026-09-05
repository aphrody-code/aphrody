<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody.com

`aphrody.com` est la vitrine web self-hosted d'Aphrody : un serveur Rust
compatible avec le moteur Codex, les modèles locaux, le RAG, l'OCR et le
répertoire persistant de l'agent (`SOUL.md`, persona, mémoire et outils).
Le serveur ne requiert aucune clé d'API ; les modèles sont chargés localement
ou via un endpoint compatible hébergé par l'opérateur.

L'ancien origin `aphrody-site` est désactivé. La future origine web est
`nie-site` du dépôt `niers`, conformément à [`niers/docs/stack`](../../niers/docs/stack/README.md).

## Déploiement

```bash
cargo build --release -p aphrody-site --target x86_64-unknown-linux-gnu
sudo install -m 755 target/x86_64-unknown-linux-gnu/release/aphrody-site ~/.local/bin/aphrody-site
sudo install -m 644 deploy/systemd/aphrody-site.service /etc/systemd/system/aphrody-site.service
sudo systemctl daemon-reload
sudo systemctl enable --now aphrody-site.service
sudo install -m 644 deploy/nginx/aphrody.com.conf /etc/nginx/conf.d/aphrody.com.conf
sudo nginx -t && sudo systemctl reload nginx
sudo certbot --nginx -d aphrody.com -d www.aphrody.com
```

DNS cible : apex `A` vers `51.77.147.152`, `www` en `CNAME` vers
`aphrody.com.`, et `api`/`downloads`/`mcp`/`cdn`/`bot`/`admin`/`bxc`/`nie` en
`CNAME` vers l'apex. Les enregistrements MX, SPF, DKIM, DMARC et autodiscover OVH doivent
être conservés. Les sous-domaines réservés servent la page blanche tant que
leur service public n'est pas explicitement activé.

Contact public : `contact@aphrody.com`.
