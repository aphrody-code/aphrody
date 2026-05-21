// SPDX-License-Identifier: Apache-2.0
// Copyright AGNTCY Contributors (https://github.com/agntcy)
//
// Server-side command registry. Ports the design of
// `packages/gemini-cli/packages/a2a-server/src/commands/`:
//
//   - `command-registry.ts`  → `CommandRegistry`
//   - `types.ts`             → `Command` trait, `CommandArgument`, `CommandContext`
//   - `extensions.ts`/`init.ts`/`memory.ts`/`restore.ts` → concrete impls
//     out of scope (those depend on gemini-cli-core internals); this
//     module supplies the framework so adopters can register their own.
//
// Commands are referenced over HTTP by `/listCommands` (GET) and
// `/executeCommand` (POST). The HTTP wiring lives in [`crate::ext_routes`];
// this module is the transport-agnostic core so commands stay testable
// without spinning up an HTTP server.

use std::{collections::HashMap, sync::Arc};

use a2a::A2AError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;

/// One positional argument descriptor surfaced via `/listCommands`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandArgument {
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_required: Option<bool>,
}

/// Result envelope returned by `/executeCommand`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecutionResponse {
    pub name: String,
    pub data: Value,
}

/// Public command listing record (serialised by `/listCommands`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDescriptor {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub arguments: Vec<CommandArgument>,
    #[serde(default)]
    pub sub_commands: Vec<CommandDescriptor>,
    #[serde(default)]
    pub requires_workspace: bool,
    #[serde(default)]
    pub streaming: bool,
}

/// Command trait. Implementations carry pure business logic and stay
/// independent of any HTTP framework.
#[async_trait]
pub trait Command: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn arguments(&self) -> &[CommandArgument] {
        &[]
    }
    fn sub_commands(&self) -> Vec<Arc<dyn Command>> {
        Vec::new()
    }
    fn top_level(&self) -> bool {
        true
    }
    fn requires_workspace(&self) -> bool {
        false
    }
    fn streaming(&self) -> bool {
        false
    }
    async fn execute(&self, args: Vec<String>) -> Result<CommandExecutionResponse, A2AError>;
}

/// Concurrent command registry. Mirrors `CommandRegistry` from
/// `commands/command-registry.ts`. Sub-commands are flattened during
/// registration so `get("memory show")` works directly.
pub struct CommandRegistry {
    commands: RwLock<HashMap<String, Arc<dyn Command>>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        CommandRegistry { commands: RwLock::new(HashMap::new()) }
    }

    /// Register a command and (recursively) any sub-commands. Returns
    /// `false` if the name was already taken.
    pub async fn register(&self, command: Arc<dyn Command>) -> bool {
        let mut store = self.commands.write().await;
        if store.contains_key(command.name()) {
            tracing::warn!(name = command.name(), "command already registered, skipping");
            return false;
        }
        let name = command.name().to_string();
        let subs = command.sub_commands();
        store.insert(name, command);
        drop(store);
        for sub in subs {
            // Recurse without holding the lock to avoid re-entrance issues.
            Box::pin(self.register(sub)).await;
        }
        true
    }

    /// Fetch a command by name.
    pub async fn get(&self, name: &str) -> Option<Arc<dyn Command>> {
        self.commands.read().await.get(name).cloned()
    }

    /// List every registered command (flat, includes sub-commands).
    pub async fn all(&self) -> Vec<Arc<dyn Command>> {
        self.commands.read().await.values().cloned().collect()
    }

    /// Render the top-level catalog as `CommandDescriptor`s. Mirrors
    /// `/listCommands` in `http/app.ts` — recurses into sub-commands,
    /// dedupes by name to avoid cycles.
    pub async fn catalog(&self) -> Vec<CommandDescriptor> {
        let all = self.all().await;
        let mut visited: Vec<String> = Vec::new();
        all.iter()
            .filter(|c| c.top_level())
            .filter_map(|c| describe(c.as_ref(), &mut visited))
            .collect()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn describe(command: &dyn Command, visited: &mut Vec<String>) -> Option<CommandDescriptor> {
    let name = command.name().to_string();
    if visited.iter().any(|v| v == &name) {
        return None;
    }
    visited.push(name.clone());
    let subs = command
        .sub_commands()
        .iter()
        .filter_map(|sub| describe(sub.as_ref(), visited))
        .collect();
    Some(CommandDescriptor {
        name,
        description: command.description().to_string(),
        arguments: command.arguments().to_vec(),
        sub_commands: subs,
        requires_workspace: command.requires_workspace(),
        streaming: command.streaming(),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    struct EchoCommand;

    #[async_trait]
    impl Command for EchoCommand {
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "Echo the joined arguments"
        }
        fn arguments(&self) -> &[CommandArgument] {
            const ARGS: &[CommandArgument] = &[];
            ARGS
        }
        async fn execute(&self, args: Vec<String>) -> Result<CommandExecutionResponse, A2AError> {
            Ok(CommandExecutionResponse {
                name: self.name().to_string(),
                data: json!({ "echo": args.join(" ") }),
            })
        }
    }

    struct ParentCommand;
    struct ChildCommand;

    #[async_trait]
    impl Command for ChildCommand {
        fn name(&self) -> &str {
            "parent child"
        }
        fn description(&self) -> &str {
            "Sub-command"
        }
        fn top_level(&self) -> bool {
            false
        }
        async fn execute(&self, _: Vec<String>) -> Result<CommandExecutionResponse, A2AError> {
            Ok(CommandExecutionResponse { name: self.name().into(), data: json!("child") })
        }
    }

    #[async_trait]
    impl Command for ParentCommand {
        fn name(&self) -> &str {
            "parent"
        }
        fn description(&self) -> &str {
            "Parent"
        }
        fn sub_commands(&self) -> Vec<Arc<dyn Command>> {
            vec![Arc::new(ChildCommand)]
        }
        async fn execute(&self, _: Vec<String>) -> Result<CommandExecutionResponse, A2AError> {
            Ok(CommandExecutionResponse { name: self.name().into(), data: json!("parent") })
        }
    }

    #[tokio::test]
    async fn test_register_and_execute() {
        let reg = CommandRegistry::new();
        assert!(reg.register(Arc::new(EchoCommand)).await);
        let cmd = reg.get("echo").await.expect("present");
        let result = cmd.execute(vec!["hello".into(), "world".into()]).await.unwrap();
        assert_eq!(result.data, json!({ "echo": "hello world" }));
    }

    #[tokio::test]
    async fn test_duplicate_registration_rejected() {
        let reg = CommandRegistry::new();
        assert!(reg.register(Arc::new(EchoCommand)).await);
        assert!(!reg.register(Arc::new(EchoCommand)).await);
    }

    #[tokio::test]
    async fn test_sub_commands_registered_recursively() {
        let reg = CommandRegistry::new();
        reg.register(Arc::new(ParentCommand)).await;
        assert!(reg.get("parent").await.is_some());
        assert!(reg.get("parent child").await.is_some());
    }

    #[tokio::test]
    async fn test_catalog_lists_top_level_with_subcommands() {
        let reg = CommandRegistry::new();
        reg.register(Arc::new(ParentCommand)).await;
        reg.register(Arc::new(EchoCommand)).await;
        let cat = reg.catalog().await;
        assert_eq!(cat.len(), 2);
        let parent = cat.iter().find(|c| c.name == "parent").unwrap();
        assert_eq!(parent.sub_commands.len(), 1);
        assert_eq!(parent.sub_commands[0].name, "parent child");
    }

    #[tokio::test]
    async fn test_missing_command_returns_none() {
        let reg = CommandRegistry::new();
        assert!(reg.get("nope").await.is_none());
    }
}
