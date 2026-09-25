// Atlas OS — Security & Compliance engine (RFC 18, Phase 7).
//
// Sub-fase 7.0: skill checksums SHA-256 (`signature`). Sub-fase 7.1:
// sandbox levels + approval policy (`sandbox`). Sub-fase 7.3:
// deterministic supply-chain install gate (`supply_gate` — Socket/Snyk
// SaaS stay a documented follow-up). Compliance skills (7.2) ship as
// bundled prompt skills under `skills/atlas-*-check/`.

pub mod sandbox;
pub mod signature;
pub mod supply_gate;

pub use sandbox::{approval_for, approval_for_str, Approval, SandboxLevel, SensitiveAction};
pub use signature::{
    catalog_checksum, install_gate, read_sidecar, skill_checksum, verify_against_sidecar,
    verify_skill, write_sidecar, ChecksumVerdict, MANIFEST_NAME, README_NAME, SIDECAR_NAME,
};
pub use supply_gate::{
    evaluate_package, evaluate_package_with_known, SupplyReport, SupplyVerdict, ENV_ACCESS_MARKERS,
    INSTALL_SCRIPTS, KNOWN_PACKAGES,
};
