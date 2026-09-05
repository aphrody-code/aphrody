<!-- SPDX-License-Identifier: Apache-2.0 -->
# Messagerie Aphrody

## État vérifié

`contact@aphrody.com` est l'identité d'envoi et de réception prévue. La zone
publique utilise actuellement OVH Mail :

- MX : `mx0.mail.ovh.net` (priorité 1), `mx1.mail.ovh.net` (5),
  `mx2.mail.ovh.net` (50), `mx3.mail.ovh.net` (100) ;
- SPF : `v=spf1 include:mx.ovh.com ~all` ;
- DMARC : `v=DMARC1; p=none; adkim=s; aspf=s` ;
- SMTP : `ssl0.ovh.net:465` (TLS implicite) ou `:587` (STARTTLS) ;
- IMAP : `ssl0.ovh.net:993` (TLS implicite).

Les ports SMTP et IMAP ont été vérifiés joignables depuis le VPS. Cela ne
prouve pas que la boîte existe : l'API utilisée par `ovhcloud` refuse les
surfaces Email Domain, MX Plan et Email Pro avec `403 This call has not been
granted`. La création et le test authentifié de la boîte restent donc bloqués.

## Secrets d'exécution

Ne jamais versionner le mot de passe de la boîte. Le service consommateur doit
charger un fichier d'environnement détenu par son utilisateur, mode `0600` :

```dotenv
MAIL_FROM="Aphrody <contact@aphrody.com>"
SMTP_HOST=ssl0.ovh.net
SMTP_PORT=465
SMTP_SECURE=true
SMTP_USER=contact@aphrody.com
SMTP_PASSWORD=
IMAP_HOST=ssl0.ovh.net
IMAP_PORT=993
IMAP_SECURE=true
IMAP_USER=contact@aphrody.com
IMAP_PASSWORD=
```

Le mot de passe fourni à l'opérateur ne doit être injecté qu'après confirmation
de l'existence de la boîte. Aucun secret mail n'est actuellement ajouté au
dépôt ou à une unité systemd.

## Better Auth

Better Auth ne transporte pas lui-même les messages. Les callbacks
`sendVerificationEmail` et `sendResetPassword` appellent un adaptateur mail
unique, alimenté par les variables ci-dessus. L'envoi doit être attendu hors de
la transaction HTTP ou placé dans une file persistante, sans journaliser
l'adresse, le lien ou le jeton.

```ts
export const auth = betterAuth({
  emailAndPassword: {
    enabled: true,
    requireEmailVerification: true,
    sendResetPassword: async ({ user, url }) => {
      await mail.sendPasswordReset({ to: user.email, url });
    },
  },
  emailVerification: {
    sendOnSignUp: true,
    sendVerificationEmail: async ({ user, url }) => {
      await mail.sendVerification({ to: user.email, url });
    },
  },
});
```

Exigences communes aux vitrines Aphrody, BXC, Niers et N2B : URL Better Auth
propre à chaque origine, secrets de session distincts, même adaptateur mail,
liens HTTPS absolus, expiration courte, usage unique et limitation de débit par
compte et par adresse IP.

## Resend

Aucune variable `RESEND_API_KEY` exploitable n'a été trouvée. Resend est donc
une option d'envoi transactionnel, pas la réception principale. Avant de
l'activer : vérifier le domaine chez Resend, publier ses DKIM et SPF en
fusionnant le SPF existant (un domaine ne doit avoir qu'un seul enregistrement
SPF), puis stocker la clé hors Git. Les MX OVH doivent rester en place tant que
la réception demeure chez OVH.

## Validation de mise en production

1. Créer ou confirmer la boîte `contact@aphrody.com` dans OVH Manager/API.
2. Tester l'authentification IMAP 993 et SMTP 465 sans afficher le secret.
3. Envoyer un message vers une boîte externe et vérifier SPF, DKIM et DMARC.
4. Répondre vers `contact@aphrody.com` et confirmer la réception IMAP.
5. Tester les parcours Better Auth de vérification et de réinitialisation.
6. Passer DMARC de `p=none` à `quarantine`, puis `reject`, après observation.

