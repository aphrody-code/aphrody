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
  MdTextButton,
} from "@aphrody/m3-react";
import { api } from "../../api/client.ts";
import { useConfig } from "../../api/queries.ts";
import { session } from "../../store.ts";

export function AuthScreen() {
  const { data: config } = useConfig();
  const navigate = useNavigate();
  const [mode, setMode] = useState<"signin" | "signup">("signin");
  const [name, setName] = useState("");
  const [email, setEmail] = useState("ada@example.com");
  const [password, setPassword] = useState("password");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const submit = async () => {
    setBusy(true);
    setError("");
    try {
      const user =
        mode === "signin"
          ? await api.signIn(email, password)
          : await api.signUp(name || "New User", email, password);
      session.signIn(user);
      void navigate({ to: "/", replace: true });
    } catch (e) {
      setError(e instanceof Error ? e.message : "Authentication failed");
    } finally {
      setBusy(false);
    }
  };

  const providers = Object.entries(config?.oauth.providers ?? {});

  return (
    <div className="owui-auth">
      <MdElevatedCard className="owui-auth__card">
        <div style={{ textAlign: "center" }}>
          <MdIcon
            style={{ fontSize: 40, color: "var(--md-sys-color-primary)" } as React.CSSProperties}
          >
            forum
          </MdIcon>
          <h1 style={{ margin: "6px 0 0", fontSize: 24 }}>{config?.name ?? "Open WebUI"}</h1>
          <p className="owui-muted" style={{ margin: "2px 0 0" }}>
            {mode === "signin" ? "Welcome back" : "Create your account"}
          </p>
        </div>

        <div className="owui-stack">
          {mode === "signup" && (
            <MdOutlinedTextField
              label="Name"
              value={name}
              onInput={(e) => setName((e.target as HTMLInputElement).value)}
            />
          )}
          <MdOutlinedTextField
            label="Email"
            type="email"
            value={email}
            onInput={(e) => setEmail((e.target as HTMLInputElement).value)}
          />
          <MdOutlinedTextField
            label="Password"
            type="password"
            value={password}
            onInput={(e) => setPassword((e.target as HTMLInputElement).value)}
          />
        </div>

        {error && (
          <p style={{ color: "var(--md-sys-color-error)", margin: 0, fontSize: 13 }}>{error}</p>
        )}

        <MdFilledButton onClick={() => void submit()} disabled={busy}>
          {busy ? (
            <MdCircularProgress indeterminate slot="icon" />
          ) : (
            <MdIcon slot="icon">login</MdIcon>
          )}
          {mode === "signin" ? "Sign in" : "Sign up"}
        </MdFilledButton>

        {providers.length > 0 && (
          <>
            <div className="owui-row" style={{ justifyContent: "center" }}>
              <span className="owui-muted" style={{ fontSize: 12 }}>
                or continue with
              </span>
            </div>
            <div className="owui-stack">
              {providers.map(([key, label]) => (
                <MdOutlinedButton key={key} onClick={() => void submit()}>
                  <MdIcon slot="icon">account_circle</MdIcon>
                  {label}
                </MdOutlinedButton>
              ))}
            </div>
          </>
        )}

        {config?.features.enable_signup && (
          <div style={{ textAlign: "center" }}>
            <MdTextButton onClick={() => setMode((m) => (m === "signin" ? "signup" : "signin"))}>
              {mode === "signin" ? "Need an account? Sign up" : "Have an account? Sign in"}
            </MdTextButton>
          </div>
        )}
      </MdElevatedCard>
    </div>
  );
}
