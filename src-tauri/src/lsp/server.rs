// Atlas OS — Minimal real LSP server (Phase 19).
// Runs over stdio when invoked from an editor or CLI (`ATLAS_LSP_STDIO=1`);
// desktop embedding parks unless a pipe host is detected (see `lsp::host`).

use std::sync::Arc;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::core::state::AppState;

/// Backend served over stdio. Holds the client sink + app state so future
/// methods can read the journal / bus without re-plumbing the connection.
pub struct AtlasLspBackend {
    pub client: Client,
    pub state: Arc<AppState>,
}

#[tower_lsp::async_trait]
impl LanguageServer for AtlasLspBackend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: atlas_capabilities(),
            server_info: Some(ServerInfo {
                name: "atlas-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "atlas-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn hover(&self, _: HoverParams) -> Result<Option<Hover>> {
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(hover_markdown())),
            range: None,
        }))
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }
}

/// Build the std-only backend (useful for tests): reused by `host::serve`.
pub fn build_backend(client: Client, state: Arc<AppState>) -> AtlasLspBackend {
    AtlasLspBackend { client, state }
}

/// Serve the backend over stdin/stdout (Block until transport closes).
pub async fn serve_stdio(state: Arc<AppState>) {
    let (service, socket) = LspService::new(|client| build_backend(client, state.clone()));
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
}

/// Pure capability block — assertable by unit tests without a client.
pub fn atlas_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        ..Default::default()
    }
}

/// Pure hover payload string used by `AtlasLspBackend::hover`.
pub fn hover_markdown() -> String {
    format!(
        "**Atlas OS** LSP host — version {}; diagnostics + hover only in v1.",
        env!("CARGO_PKG_VERSION")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_advertises_hover_and_full_sync() {
        let caps = atlas_capabilities();
        assert!(matches!(
            caps.hover_provider,
            Some(HoverProviderCapability::Simple(true))
        ));
        assert!(matches!(
            caps.text_document_sync,
            Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL))
        ));
    }

    #[test]
    fn hover_payload_carries_version() {
        let md = hover_markdown();
        assert!(md.contains(env!("CARGO_PKG_VERSION")));
        assert!(md.contains("Atlas OS"));
    }
}
