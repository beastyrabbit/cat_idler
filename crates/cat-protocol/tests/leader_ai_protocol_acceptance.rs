//! LAI.1 protocol cutover acceptance boundary.

use cat_protocol::PROTOCOL_VERSION;
use serde::Deserialize;

const CONTRACT_JSON: &str =
    include_str!("../../../docs/leader-ai-overhaul/fixtures/lai1_acceptance_contract.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AcceptanceContract {
    protocol: ProtocolContract,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProtocolContract {
    legacy_version: u32,
    rejection_code: String,
    mutation_fields: Vec<String>,
}

#[test]
fn leader_ai_atomic_cutover_bumps_protocol_before_replacement_payloads_ship() {
    let contract: AcceptanceContract =
        serde_json::from_str(CONTRACT_JSON).expect("LAI.1 acceptance contract");

    assert_eq!(contract.protocol.rejection_code, "UPDATE_REQUIRED");
    assert_eq!(
        contract.protocol.mutation_fields,
        ["protocolVersion", "idempotencyId", "expectedStateVersion"]
    );
    assert!(
        PROTOCOL_VERSION > contract.protocol.legacy_version,
        "replacement snapshots/actions must not ship under the legacy protocol version"
    );
}
