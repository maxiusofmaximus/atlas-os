// Atlas OS — Security & Compliance engine (RFC 18, Phase 7).
//
// Sub-fase 7.0: skill checksums SHA-256 (`signature`). Sub-fase 7.1:
// sandbox levels + approval policy (`sandbox`). Supply-chain gate and
// compliance skills land in 7.2–7.3.

pub mod sandbox;
pub mod signature;

pub use sandbox::{approval_for, approval_for_str, Approval, SandboxLevel, SensitiveAction};
pub use signature::{
    catalog_checksum, install_gate, read_sidecar, skill_checksum, verify_against_sidecar,
    verify_skill, write_sidecar, ChecksumVerdict, MANIFEST_NAME, README_NAME, SIDECAR_NAME,
};
