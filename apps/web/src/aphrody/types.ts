// Shapes shared by the React port of the aphrody desktop (Angular) app.
// Mirror the Rust/Tauri contract the Angular services used.

/** Result of an in-process aphrody CLI run (Rust `ExecResult`). */
export interface ExecResult {
  code: number;
  stdout: string;
  stderr: string;
}

/** Static shell + host metadata (Rust `Meta`). */
export interface Meta {
  app_version: string;
  target_os: string;
  target_arch: string;
  family: string;
}

/** Google account linked to aphrody, resolved live (never persisted). */
export interface Account {
  connected: boolean;
  email: string;
  name: string;
  initials: string;
}
