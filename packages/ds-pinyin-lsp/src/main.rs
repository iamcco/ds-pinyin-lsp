use dashmap::DashMap;
use ds_pinyin_lsp::sqlite::query_dict;
use ds_pinyin_lsp::types::Setting;
use ds_pinyin_lsp::utils::{
    get_pinyin, get_pre_line, long_suggests_to_completion_item, query_long_sentence,
    suggests_to_completion_item, symbols_to_completion_item,
};
use lsp_document::{apply_change, IndexedText, TextAdapter};
use rusqlite::Connection;
use serde_json::Value;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
    setting: Mutex<Setting>,
    conn: Mutex<Option<Connection>>,
    documents: DashMap<String, IndexedText<String>>,
    symbols: DashMap<char, Vec<String>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        self.init(&params.initialization_options).await;

        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(
                        self.symbols
                            .iter()
                            .map(|s| s.key().to_string())
                            .collect::<Vec<String>>(),
                    ),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                }),
                ..ServerCapabilities::default()
            },
        })
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
        // check completion on/off
        let setting = self.setting.lock().await;
        if !setting.completion_on {
            return Ok(Some(CompletionResponse::Array(vec![])));
        }

        let position = params.text_document_position.position;
        let uri = params.text_document_position.text_document.uri.to_string();
        let document = self.documents.get(&uri);
        let pre_line = get_pre_line(&document, &position).unwrap_or("");

        if pre_line.is_empty() {
            return Ok(Some(CompletionResponse::Array(vec![])));
        }

        let pinyin = get_pinyin(pre_line).unwrap_or(String::new());

        if pinyin.is_empty() {
            // check symbol
            if let Some(last_char) = pre_line.chars().last() {
                if let Some(symbols) = self.symbols.get(&last_char) {
                    return Ok(Some(CompletionResponse::List(CompletionList {
                        is_incomplete: true,
                        items: symbols_to_completion_item(last_char, symbols, position),
                    })));
                }
            }

            return Ok(Some(CompletionResponse::Array(vec![])));
        }

        // pinyin range
        let range = Range::new(
            Position {
                line: position.line,
                character: position.character - pinyin.len() as u32,
            },
            position,
        );

        if let Some(ref conn) = *self.conn.lock().await {
            // dict search match
            if let Ok(suggests) = query_dict(conn, &pinyin, 50) {
                if suggests.len() > 0 {
                    return Ok(Some(CompletionResponse::List(CompletionList {
                        is_incomplete: true,
                        items: suggests_to_completion_item(suggests, range),
                    })));
                }
            }

            // long sentence
            if let Ok(Some(suggests)) = query_long_sentence(conn, &pinyin) {
                if suggests.len() > 0 {
                    return Ok(Some(CompletionResponse::List(CompletionList {
                        is_incomplete: true,
                        items: long_suggests_to_completion_item(suggests, range),
                    })));
                }
            }
        };

        Ok(Some(CompletionResponse::Array(vec![])))
    }
}

impl Backend {
    async fn turn_off_completion(&self) {
        let mut setting = self.setting.lock().await;
        (*setting).completion_on = false
    }

    async fn turn_on_completion(&self) {
        let mut setting = self.setting.lock().await;
        (*setting).completion_on = true
    }

    async fn init(&self, initialization_options: &Option<Value>) {
        if let Some(params) = initialization_options {
            let mut setting = self.setting.lock().await;

            let db_path = &Value::String(String::new());

            let db_path = params.get("db-path").unwrap_or(&db_path);

            // invalid db_path
            if !db_path.is_string() {
                return self
                    .client
                    .show_message(MessageType::ERROR, "ds-pinyin-lsp db-path must be string!")
                    .await;
            }

            if let Some(db_path) = db_path.as_str() {
                // db_path missing
                if db_path.is_empty() {
                    return self
                        .client
                        .show_message(MessageType::ERROR, "ds-pinyin-lsp db-path is empty string!")
                        .await;
                }

                // cache setting
                (*setting).db_path = Some(db_path.to_string());

                // open db connection
                let conn = Connection::open(db_path);
                if let Ok(conn) = conn {
                    let mut mutex = self.conn.lock().await;
                    *mutex = Some(conn);
                    return self
                        .client
                        .log_message(
                            MessageType::INFO,
                            "ds-pinyin-lsp db connection initialized!",
                        )
                        .await;
                } else if let Err(err) = conn {
                    return self
                        .client
                        .show_message(MessageType::ERROR, &format!("Open database error: {}", err))
                        .await;
                }
            }
        } else {
            return self
                .client
                .show_message(
                    MessageType::ERROR,
                    "ds-pinyin-lsp initialization_options is missing, it must include db-path setting!",
                )
                .await;
        }
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let symbols = [
        ('.', vec!["。", "·", "……"]),
        ('`', vec!["·", "～"]),
        ('\\', vec!["、"]),
        (',', vec!["，"]),
        (';', vec!["；"]),
        (':', vec!["："]),
        ('?', vec!["？"]),
        ('!', vec!["！"]),
        ('\"', vec!["“", "”"]),
        ('\'', vec!["‘", "’"]),
        ('(', vec!["（"]),
        (')', vec!["）"]),
        ('-', vec!["——"]),
        ('<', vec!["《"]),
        ('>', vec!["》"]),
        ('[', vec!["【"]),
        (']', vec!["】"]),
        ('$', vec!["¥"]),
    ]
    .into_iter()
    .map(|s| {
        (
            s.0,
            s.1.into_iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
        )
    })
    .collect::<DashMap<char, Vec<String>>>();

    let (service, socket) = LspService::build(|client| Backend {
        client,
        setting: Mutex::new(Setting::new()),
        conn: Mutex::new(None),
        documents: DashMap::new(),
        symbols,
    })
    .custom_method("Turn/off/completion", Backend::turn_off_completion)
    .custom_method("Turn/on/completion", Backend::turn_on_completion)
    .finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}
