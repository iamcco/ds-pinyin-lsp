use dashmap::DashMap;
use lsp_document::{apply_change, IndexedText, TextAdapter};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
    documents: DashMap<String, IndexedText<String>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: None,
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                }),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "ds-pinyin-lsp initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.documents.insert(
            params.text_document.uri.to_string(),
            IndexedText::new(params.text_document.text),
        );
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let mut document = self
            .documents
            .entry(params.text_document.uri.to_string())
            .or_insert(IndexedText::new(String::new()));

        let mut content: String;

        for change in params.content_changes {
            if let Some(change) = document.lsp_change_to_change(change) {
                content = apply_change(&document, change);
                *document = IndexedText::new(content);
            }
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();

        // remove close document
        self.documents.remove(&uri);

        self.client
            .log_message(MessageType::INFO, &format!("Close file: {}", &uri))
            .await;
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        // let uri = params.text_document_position.text_document.uri;
        // let position = params.text_document_position.position;
        Ok(Some(vec![]).map(CompletionResponse::Array))
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(|client| Backend {
        client,
        documents: DashMap::new(),
    })
    .finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}
