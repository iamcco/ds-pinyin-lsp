use dashmap::DashMap;
use ds_pinyin_lsp::utils::{get_pinyin, get_pre_line, query_dict, query_words};
use lsp_document::{apply_change, IndexedText, TextAdapter};
use rusqlite::Connection;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
    conn: Mutex<Option<Connection>>,
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
                    resolve_provider: Some(true),
                    trigger_characters: Some(vec![]),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                }),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        let conn = Connection::open(
            "/Users/aioiyuuko/develop/pinyin-lsp/packages/dict-builder/dicts/dict.db3",
        );
        if let Ok(conn) = conn {
            let mut mutex = self.conn.lock().await;
            *mutex = Some(conn);
            self.client
                .log_message(MessageType::INFO, "ds-pinyin-lsp initialized!")
                .await;
        } else if let Err(err) = conn {
            self.client
                .show_message(MessageType::INFO, &format!("Open database error: {}", err))
                .await;
        }
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

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let position = params.text_document_position.position;
        let uri = params.text_document_position.text_document.uri.to_string();
        let document = self.documents.get(&uri);
        let pre_line = get_pre_line(&document, &position).unwrap_or("");

        if pre_line.is_empty() {
            return Ok(Some(vec![]).map(CompletionResponse::Array));
        }

        let pinyin = get_pinyin(pre_line).unwrap_or(String::new());

        if pinyin.is_empty() {
            return Ok(Some(vec![]).map(CompletionResponse::Array));
        }

        self.client
            .log_message(MessageType::INFO, &format!("pinyin: {}", &pinyin))
            .await;

        if let Some(ref conn) = *self.conn.lock().await {
            // words match
            if let Ok(suggest) = query_words(conn, &pinyin, pinyin.len() > 3) {
                if suggest.len() > 0 {
                    let res = suggest
                        .into_iter()
                        .map(|s| CompletionItem {
                            label: s.hanzi,
                            kind: Some(CompletionItemKind::TEXT),
                            filter_text: Some(s.pinyin),
                            ..Default::default()
                        })
                        .collect::<Vec<CompletionItem>>();
                    return Ok(Some(CompletionResponse::List(CompletionList {
                        is_incomplete: true,
                        items: res,
                    })));
                }
            }

            // dict suggest
            if let Ok(suggest) = query_dict(conn, &pinyin) {
                if suggest.len() > 0 {
                    let res = suggest
                        .into_iter()
                        .map(|s| CompletionItem {
                            label: s.hanzi,
                            kind: Some(CompletionItemKind::TEXT),
                            filter_text: Some(s.pinyin),
                            ..Default::default()
                        })
                        .collect::<Vec<CompletionItem>>();
                    return Ok(Some(CompletionResponse::List(CompletionList {
                        is_incomplete: true,
                        items: res,
                    })));
                }
            }
        };

        Ok(Some(CompletionResponse::List(CompletionList {
            is_incomplete: false,
            items: vec![],
        })))
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(|client| Backend {
        client,
        conn: Mutex::new(None),
        documents: DashMap::new(),
    })
    .finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}
