// SPDX-License-Identifier: Apache-2.0
use a2a::{
    AgentCapabilities, AgentCard, AgentInterface, AgentSkill, TRANSPORT_PROTOCOL_JSONRPC, VERSION,
};
use a2a_server::WELL_KNOWN_AGENT_CARD_PATH;

use crate::manifest::AiManifest;

/// Build an A2A 1.0 [`AgentCard`] from [`AiManifest`] + listener base URL.
#[must_use]
pub fn agent_card_from_manifest(manifest: &AiManifest, base_url: &str) -> AgentCard {
    let cap = manifest.capabilities.clone().unwrap_or_default();
    let skills: Vec<AgentSkill> = manifest
        .skills
        .iter()
        .map(|s| AgentSkill {
            id: s.id.clone(),
            name: s.name.clone(),
            description: s.description.clone().unwrap_or_default(),
            tags: s.tags.clone(),
            examples: None,
            input_modes: Some(manifest.default_input_modes.clone()),
            output_modes: Some(manifest.default_output_modes.clone()),
            security_requirements: None,
        })
        .collect();

    let jsonrpc_url = format!("{base_url}");
    AgentCard {
        name: manifest.name.clone().unwrap_or_else(|| "aphrody".to_owned()),
        description: manifest
            .description
            .clone()
            .unwrap_or_else(|| "Aphrody A2A agent — file JSONL + HTTP JSON-RPC".to_owned()),
        version: manifest.version.clone(),
        supported_interfaces: vec![AgentInterface::new(jsonrpc_url, TRANSPORT_PROTOCOL_JSONRPC)],
        capabilities: AgentCapabilities {
            streaming: Some(cap.streaming),
            push_notifications: Some(cap.push_notifications),
            extended_agent_card: Some(cap.extended_agent_card),
            extensions: None,
        },
        default_input_modes: if manifest.default_input_modes.is_empty() {
            vec!["text/plain".to_owned()]
        } else {
            manifest.default_input_modes.clone()
        },
        default_output_modes: if manifest.default_output_modes.is_empty() {
            vec!["text/plain".to_owned(), "application/json".to_owned()]
        } else {
            manifest.default_output_modes.clone()
        },
        skills,
        provider: manifest.provider.as_ref().map(|p| a2a::AgentProvider {
            organization: p.organization.clone(),
            url: p
                .url
                .clone()
                .unwrap_or_else(|| "https://github.com/aphrody-code/aphrody".to_owned()),
        }),
        documentation_url: manifest.documentation_url.clone(),
        icon_url: None,
        security_schemes: None,
        security_requirements: None,
        signatures: None,
    }
}

/// Well-known path constant re-export for docs/tests.
pub const AGENT_CARD_PATH: &str = WELL_KNOWN_AGENT_CARD_PATH;

/// Protocol version string advertised on the wire.
pub const WIRE_VERSION: &str = VERSION;