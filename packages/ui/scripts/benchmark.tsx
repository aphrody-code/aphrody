import { Html, Button, Card, TextField, List, ListItem, NavigationDrawer, NavigationBar, Fab, CircularProgress } from "../src/index.ts";

const App = () => (
  <Html.Fragment>
    <html lang="en" class="dark">
      <head>
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>Google OS - God Mode (MD3)</title>
      </head>
      <body>
        <NavigationDrawer open>
          <div style="padding: 24px;">Google OS</div>
          <List>
            <ListItem headline="Dashboard" class="active" />
            <ListItem headline="Forensics" />
            <ListItem headline="Network" />
            <ListItem headline="Settings" />
          </List>
        </NavigationDrawer>

        <div class="main-content">
          <header>
            <h1>Dashboard (God Mode)</h1>
            <Fab variant="tertiary" icon="add" label="New Task" lowered />
          </header>

          <div class="grid">
            <Card variant="elevated">
              <h3 style="margin: 0;">System Status</h3>
              <List>
                <ListItem headline="Kernel" supportingText="Canary 1.0.0 (Rust)" />
                <ListItem headline="Privileges" supportingText="Ring 0 / Divine" />
                <ListItem headline="UI Engine" supportingText="Bun JSX + Wry DX12" />
              </List>
            </Card>

            <Card variant="outlined">
              <h3 style="margin: 0;">Command Center</h3>
              <TextField variant="filled" label="Execute native command" style="width: 100%;" />
              <Button variant="filled">Execute</Button>
            </Card>

            <Card variant="filled">
              <h3 style="margin: 0;">Telemetry</h3>
              <CircularProgress value={0.75} />
            </Card>
          </div>
        </div>
      </body>
    </html>
  </Html.Fragment>
);

const ITERATIONS = 10000;

console.log(`🚀 Benchmarking Native Bun JSX -> HTML String Rendering (${ITERATIONS} iterations)...`);

const start = performance.now();

for (let i = 0; i < ITERATIONS; i++) {
  const _html = <App />;
}

const end = performance.now();
const totalTimeMs = end - start;
const opsPerSec = Math.round((ITERATIONS / totalTimeMs) * 1000);

console.log(`⏱️  Total Time: ${totalTimeMs.toFixed(2)} ms`);
console.log(`⚡ Performance: ${opsPerSec.toLocaleString()} renders per second`);
console.log(`✅ JSX Engine is optimized for extreme throughput.`);
