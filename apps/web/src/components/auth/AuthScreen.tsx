// "/auth" — full-screen M3 sign-in / sign-up with OAuth buttons sourced from
// the backend config. Authenticates against the mock backend, then enters the app.

import { useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import {
  MdCircularProgress,
  MdElevatedCard,
  MdFilledButton,
  MdIcon,
  MdOutlinedButton,
  MdOutlinedTextField,
} from "@aphrody/m3-react";
import { api, auth } from "../../api/client.ts";
import { useConfig } from "../../api/queries.ts";
import { session } from "../../store.ts";

export function AuthScreen() {
  const { data: config } = useConfig();
  const navigate = useNavigate();
  const [token, setToken] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const submit = async () => {
    if (token.length !== 16) {
      setError("Le token doit faire exactement 16 caracteres.");
      return;
    }
    setBusy(true);
    setError("");
    try {
      auth.set(token);
      const user = await api.getSession();
      session.signIn({ ...user, token });
      void navigate({ to: "/", replace: true });
    } catch (e) {
      auth.clear();
      setError("Token invalide ou serveur injoignable.");
    } finally {
      setBusy(false);
    }
  };

  const loginWithGoogle = async () => {
    setBusy(true);
    setError("");
    try {
      const res = await fetch("/api/auths/google", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
      });
      if (!res.ok) {
        throw new Error("Erreur de connexion Google");
      }
      const data = await res.json();
      if (data.user && data.user.token) {
        auth.set(data.user.token);
        session.signIn(data.user);
        void navigate({ to: "/", replace: true });
      } else {
        throw new Error("Données de connexion invalides");
      }
    } catch (e) {
      auth.clear();
      setError("Connexion Google échouée ou serveur injoignable.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="owui-auth">
      <MdElevatedCard className="owui-auth__card">
        <div style={{ textAlign: "center" }}>
          <MdIcon
            style={{ fontSize: 40, color: "var(--md-sys-color-primary)" } as React.CSSProperties}
          >
            lock
          </MdIcon>
          <h1 style={{ margin: "6px 0 0", fontSize: 24 }}>{config?.name ?? "Open WebUI"}</h1>
          <p className="owui-muted" style={{ margin: "2px 0 0" }}>
            Veuillez entrer votre token d'acces a 16 caracteres
          </p>
        </div>

        <div className="owui-stack">
          <MdOutlinedTextField
            label="Token d'Acces"
            type="password"
            value={token}
            onInput={(e) => setToken((e.target as HTMLInputElement).value)}
            maxLength={16}
          />
        </div>

        {error && (
          <p style={{ color: "var(--md-sys-color-error)", margin: 0, fontSize: 13 }}>{error}</p>
        )}

        <div className="owui-stack" style={{ gap: 8 }}>
          <MdFilledButton onClick={() => void submit()} disabled={busy}>
            {busy ? (
              <MdCircularProgress indeterminate slot="icon" />
            ) : (
              <MdIcon slot="icon">vpn_key</MdIcon>
            )}
            Se connecter
          </MdFilledButton>

          <MdOutlinedButton onClick={() => void loginWithGoogle()} disabled={busy}>
            <MdIcon slot="icon">account_circle</MdIcon>
            Se connecter avec Google
          </MdOutlinedButton>
        </div>
      </MdElevatedCard>
    </div>
  );
}
