// Atlas OS — Domain Engines & Vertical Harnesses (RFC 64, Fase 28).
//
// A *domain pack* is a declarative bundle (not core code): engine routing,
// bundled skills, MCP servers, lateral tools and validation/artifact policy.
// The core stays generic; a domain connects through this contract. Tools are
// always *lateral* (external process, never bundled — RFC 25 §11).

pub mod lateral;
pub mod manifest;
pub mod registry;

pub use manifest::{pack_sha256, parse_manifest, DomainPack};
pub use registry::DomainRegistry;
