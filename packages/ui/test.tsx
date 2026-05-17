import { Html, Button, Card, TextField, Checkbox, Dialog, List, ListItem, NavigationBar } from "./src/index.ts";

const App = () => (
  <main class="app-container">
    <NavigationBar>
      <h1>Gemini OS</h1>
    </NavigationBar>
    <Card variant="elevated">
      <h2>Welcome to God Mode</h2>
      <TextField variant="outlined" label="Enter Command" />
      <div class="actions">
        <Button variant="filled">Execute</Button>
        <Button variant="text">Cancel</Button>
      </div>
    </Card>
    <List>
      <ListItem headline="System Status" supportingText="All systems operational" />
      <ListItem headline="Privileges" supportingText="Ring 0 (Divine)" />
    </List>
    <Checkbox checked />
  </main>
);

const htmlOutput = <App />;
console.log("--- GENERATED HTML FROM NATIVE BUN JSX ---");
console.log(htmlOutput);
console.log("------------------------------------------");
