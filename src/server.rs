use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::completion::get_completions;
use crate::diagnostics::validate_document;
use crate::hover::get_hover;
use crate::parser::LineDocument;

pub struct EnvSpecLsp {
    client: Client,
    documents: Arc<RwLock<HashMap<url::Url, String>>>,
}

impl EnvSpecLsp {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for EnvSpecLsp {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(
                        ["@", "$", "=", "(", ","]
                            .iter()
                            .map(|s| s.to_string())
                            .collect(),
                    ),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "varlock-lsp".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "varlock-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        {
            let mut docs = self.documents.write().await;
            docs.insert(params.text_document.uri.clone(), params.text_document.text);
        }
        self.publish_diagnostics(&params.text_document.uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.last() {
            {
                let mut docs = self.documents.write().await;
                docs.insert(params.text_document.uri.clone(), change.text.clone());
            }
            self.publish_diagnostics(&params.text_document.uri).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        {
            let mut docs = self.documents.write().await;
            docs.remove(&params.text_document.uri);
        }
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let docs = self.documents.read().await;
        let text = match docs.get(&params.text_document_position.text_document.uri) {
            Some(t) => t,
            None => return Ok(None),
        };

        let doc = LineDocument::new(text);
        let position = params.text_document_position.position;

        Ok(
            get_completions(&doc, position).map(|items| {
                CompletionResponse::List(CompletionList {
                    is_incomplete: false,
                    items,
                })
            }),
        )
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let docs = self.documents.read().await;
        let text = match docs.get(&params.text_document_position_params.text_document.uri) {
            Some(t) => t,
            None => return Ok(None),
        };

        let doc = LineDocument::new(text);
        Ok(get_hover(&doc, params.text_document_position_params.position))
    }
}

impl EnvSpecLsp {
    async fn publish_diagnostics(&self, uri: &url::Url) {
        let docs = self.documents.read().await;
        let text = match docs.get(uri) {
            Some(t) => t,
            None => return,
        };

        let doc = LineDocument::new(text);
        let diagnostics = validate_document(&doc);

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }
}
