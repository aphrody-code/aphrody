// Vue À propos : logo aphrody, état du compte (useAccount), méta hôte (useMeta), carte version et liens utiles.

import { MdIcon, MdOutlinedCard, MdTextButton } from "@aphrody-code/m3-react";
import { useAccount, useMeta } from "../../client.ts";
import { VersionCard } from "../diagnostic/VersionCard.tsx";

interface LinkDef {
  label: string;
  icon: string;
  href: string;
}

const LINKS: LinkDef[] = [
  { label: "Dépôt GitHub", icon: "code", href: "https://github.com/aphrody-code/aphrody" },
  {
    label: "Documentation",
    icon: "menu_book",
    href: "https://github.com/aphrody-code/aphrody#readme",
  },
  {
    label: "Signaler un problème",
    icon: "bug_report",
    href: "https://github.com/aphrody-code/aphrody/issues",
  },
];

export function About() {
  const account = useAccount();
  const meta = useMeta();
  const acc = account.data;
  const m = meta.data;

  return (
    <div className="aph-about">
      <div className="aph-about__brand">
        <span className="aph-about__logo">
          <MdIcon>auto_awesome</MdIcon>
        </span>
        <h1 className="aph-about__name">aphrody</h1>
        <p className="aph-about__tagline">
          Le CLI cross-platform ultime — assistant, reverse engineering et forensics.
        </p>
      </div>

      <MdOutlinedCard className="aph-panel aph-about__account">
        <header className="aph-panel__head">
          <span className="aph-row">
            <MdIcon className="aph-panel__icon">account_circle</MdIcon>
            <h2 className="aph-panel__title">Compte</h2>
          </span>
        </header>
        <div className="aph-panel__body aph-about__account-body">
          {acc?.connected ? (
            <>
              <span className="aph-about__avatar">{acc.initials}</span>
              <div>
                <div className="aph-about__account-name">{acc.name}</div>
                <div className="aph-muted">{acc.email}</div>
              </div>
              <span className="aph-about__badge">Connecté</span>
            </>
          ) : (
            <div className="aph-muted">Aucun compte connecté.</div>
          )}
        </div>
      </MdOutlinedCard>

      <div className="aph-about__card-wrap">
        <VersionCard />
      </div>

      {m && (
        <div className="aph-about__host aph-muted">
          {m.target_os} · {m.target_arch} · {m.family} · {m.app_version}
        </div>
      )}

      <div className="aph-about__links">
        {LINKS.map((l) => (
          <MdTextButton key={l.href} href={l.href} target="_blank" rel="noopener noreferrer">
            <MdIcon slot="icon">{l.icon}</MdIcon>
            {l.label}
          </MdTextButton>
        ))}
      </div>

      <p className="aph-about__note aph-muted">
        Interface portée en React 19 + Material 3 (@aphrody-code/m3-react), propulsée en local par
        le binaire aphrody. Apparence inspirée de l'app Gemini.
      </p>
    </div>
  );
}
