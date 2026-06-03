// M3 settings dialog (md-dialog + md-tabs). Mirrors open-webui's settings modal:
// General (theme/seed/language), Interface (toggles/text-scale), Account, About.
// Theme mode + seed are live-wired to the store (re-themes the whole app instantly).

import { useState } from "react";
import {
  MdDialog,
  MdIcon,
  MdOutlinedTextField,
  MdPrimaryTab,
  MdOutlinedSelect,
  MdSelectOption,
  MdSlider,
  MdSwitch,
  MdTabs,
  MdTextButton,
} from "@aphrody/m3-react";
import { useConfig } from "../../api/queries.ts";
import { getState, setState, useUi, type ThemeMode } from "../../store.ts";

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div
      className="owui-spread"
      style={{ padding: "10px 0", borderBottom: "1px solid var(--md-sys-color-outline-variant)" }}
    >
      <span>{label}</span>
      {children}
    </div>
  );
}

export function SettingsDialog() {
  const { settingsOpen, themeMode, seed, user } = useUi();
  const { data: config } = useConfig();
  const [tab, setTab] = useState(0);
  const [textScale, setTextScale] = useState(100);
  const [system, setSystem] = useState("");
  const [autoTitle, setAutoTitle] = useState(true);
  const [bubbles, setBubbles] = useState(true);

  const close = () => setState({ settingsOpen: false });

  return (
    <MdDialog
      open={settingsOpen}
      onClosed={close}
      style={{ "--md-dialog-container-max-inline-size": "560px" } as React.CSSProperties}
    >
      <div slot="headline">Settings</div>
      <form slot="content" method="dialog" style={{ minHeight: 320 }}>
        <MdTabs
          activeTabIndex={tab}
          onChange={(e) =>
            setTab((e.target as unknown as { activeTabIndex: number }).activeTabIndex)
          }
        >
          <MdPrimaryTab>General</MdPrimaryTab>
          <MdPrimaryTab>Interface</MdPrimaryTab>
          <MdPrimaryTab>Account</MdPrimaryTab>
          <MdPrimaryTab>About</MdPrimaryTab>
        </MdTabs>

        {tab === 0 && (
          <div>
            <Row label="Theme">
              <MdOutlinedSelect
                value={themeMode}
                onChange={(e) =>
                  setState({ themeMode: (e.target as HTMLInputElement).value as ThemeMode })
                }
              >
                <MdSelectOption value="system">
                  <span slot="headline">System</span>
                </MdSelectOption>
                <MdSelectOption value="light">
                  <span slot="headline">Light</span>
                </MdSelectOption>
                <MdSelectOption value="dark">
                  <span slot="headline">Dark</span>
                </MdSelectOption>
              </MdOutlinedSelect>
            </Row>
            <Row label="Accent (Material You seed)">
              <input
                type="color"
                value={seed}
                onChange={(e) => setState({ seed: e.target.value })}
                style={{ width: 44, height: 32, border: "none", background: "none" }}
              />
            </Row>
            <Row label="Language">
              <MdOutlinedSelect value="en">
                <MdSelectOption value="en">
                  <span slot="headline">English</span>
                </MdSelectOption>
                <MdSelectOption value="fr">
                  <span slot="headline">Français</span>
                </MdSelectOption>
              </MdOutlinedSelect>
            </Row>
            <div style={{ paddingTop: 12 }}>
              <p style={{ margin: "0 0 4px" }}>System prompt</p>
              <MdOutlinedTextField
                type="textarea"
                rows={3}
                value={system}
                placeholder="You are a helpful assistant…"
                onInput={(e) => setSystem((e.target as HTMLInputElement).value)}
                style={{ width: "100%" }}
              />
            </div>
          </div>
        )}

        {tab === 1 && (
          <div>
            <Row label="Auto-generate chat titles">
              <MdSwitch
                selected={autoTitle}
                onChange={(e) =>
                  setAutoTitle((e.target as unknown as { selected: boolean }).selected)
                }
              />
            </Row>
            <Row label="Chat bubbles">
              <MdSwitch
                selected={bubbles}
                onChange={(e) =>
                  setBubbles((e.target as unknown as { selected: boolean }).selected)
                }
              />
            </Row>
            <div style={{ paddingTop: 12 }}>
              <p style={{ margin: "0 0 4px" }}>Text scale — {textScale}%</p>
              <MdSlider
                min={80}
                max={140}
                step={10}
                value={textScale}
                onInput={(e) => setTextScale(Number((e.target as HTMLInputElement).value))}
                style={{ width: "100%" }}
              />
            </div>
          </div>
        )}

        {tab === 2 && (
          <div className="owui-stack" style={{ paddingTop: 12 }}>
            <MdOutlinedTextField label="Name" value={user?.name ?? ""} />
            <MdOutlinedTextField label="Email" value={user?.email ?? ""} />
            <Row label="Role">
              <span className="owui-muted">{user?.role}</span>
            </Row>
          </div>
        )}

        {tab === 3 && (
          <div style={{ paddingTop: 16 }}>
            <p className="owui-row">
              <MdIcon>info</MdIcon>&nbsp;{config?.name}
            </p>
            <p className="owui-muted">
              Version {config?.version} · Material Design 3 · Bun + TanStack
            </p>
            <p className="owui-muted">A full M3 React rebuild of Open WebUI.</p>
          </div>
        )}
      </form>
      <div slot="actions">
        <MdTextButton
          onClick={() => {
            setState({ seed: getState().seed });
            close();
          }}
        >
          Close
        </MdTextButton>
      </div>
    </MdDialog>
  );
}
