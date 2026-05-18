// SPDX-License-Identifier: Apache-2.0
//! End-to-end smoke: discover() must succeed even on a host with zero of
//! the three agents installed, and detected entries must round-trip their
//! protocol classification consistently with the static registry.

use aphrody_design_agents::{AGENT_DEFS, AgentId, AgentRegistry, agent_def};

#[test]
fn registry_discover_smoke() {
    let registry = AgentRegistry::discover();
    // At most 3 agents detected. Cannot lower-bound — depends on host.
    assert!(registry.len() <= AGENT_DEFS.len());
    assert_eq!(AGENT_DEFS.len(), 3);
    for (id, desc) in registry.agents() {
        let def = agent_def(*id);
        assert_eq!(desc.id, id.slug());
        assert_eq!(desc.protocol, def.protocol);
        assert!(
            desc.binary_path.exists(),
            "detected binary {:?} should exist on disk",
            desc.binary_path
        );
    }
}

#[test]
fn known_agent_defs_have_consistent_metadata() {
    for def in AGENT_DEFS.iter() {
        assert!(
            !def.display_name.is_empty(),
            "{:?} missing display_name",
            def.id
        );
        assert!(!def.bin.is_empty(), "{:?} missing bin", def.id);
        assert!(
            def.version_args
                .iter()
                .any(|a| a.starts_with("--") || a == &"-v"),
            "{:?} version args should look like a flag",
            def.id
        );
    }
}

#[test]
fn exactly_three_agents_in_registry() {
    // Hard guard so a future addition cannot silently change the contract
    // that this binary is the canonical 3-CLI dispatcher.
    let ids: Vec<_> = AgentId::all().to_vec();
    assert_eq!(ids.len(), 3);
    assert!(ids.contains(&AgentId::ClaudeCode));
    assert!(ids.contains(&AgentId::Gemini));
    assert!(ids.contains(&AgentId::Antigravity));
}
