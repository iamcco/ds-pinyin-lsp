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
        if let Some(initialization_options) = params.initialization_options {
            self.change_configuration(&initialization_options).await;
        } else {
            self
                .client
                .show_message(
                    MessageType::ERROR,
                    "[ds-pinyin-lsp]: initialization_options is missing, it must include db_path setting!",
                )
                .await;
        }

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

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        self.change_configuration(&params.settings).await;
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
            if setting.show_symbols {
                // check symbol
                if let Some(last_char) = pre_line.chars().last() {
                    if let Some(symbols) = self.symbols.get(&last_char) {
                        return Ok(Some(CompletionResponse::List(CompletionList {
                            is_incomplete: true,
                            items: symbols_to_completion_item(last_char, symbols, position),
                        })));
                    }
                }
            }

            // return for empty pinyin
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
            if let Ok(suggests) = query_dict(conn, &pinyin, 50, setting.match_as_same_as_input) {
                if suggests.len() > 0 {
                    return Ok(Some(CompletionResponse::List(CompletionList {
                        is_incomplete: true,
                        items: suggests_to_completion_item(suggests, range),
                    })));
                }
            }

            // long sentence
            if setting.match_long_input {
                if let Ok(Some(suggests)) =
                    query_long_sentence(conn, &pinyin, setting.match_as_same_as_input)
                {
                    if suggests.len() > 0 {
                        return Ok(Some(CompletionResponse::List(CompletionList {
                            is_incomplete: true,
                            items: long_suggests_to_completion_item(suggests, range),
                        })));
                    }
                }
            }
        };

        // Note:
        // hack ghost item for more completion request from client
        Ok(Some(CompletionResponse::List(CompletionList {
            is_incomplete: true,
            items: vec![CompletionItem {
                label: String::from("Pinyin Placeholder"),
                kind: Some(CompletionItemKind::TEXT),
                filter_text: Some(String::from("‍")),
                ..Default::default()
            }],
        })))
    }
}

impl Backend {
    async fn turn_completion(&self, params: Value) {
        let mut setting = self.setting.lock().await;

        if let Some(completion_on) = params.get("completion_on") {
            if completion_on.is_boolean() {
                (*setting).completion_on = completion_on.as_bool().unwrap_or(setting.completion_on);
            }
        } else {
            (*setting).completion_on = !setting.completion_on;
        }

        self.client
            .log_message(
                MessageType::INFO,
                &format!("[ds-pinyin-lsp]: completion_on: {}", setting.completion_on),
            )
            .await;
    }

    async fn change_configuration(&self, params: &Value) {
        let mut setting = self.setting.lock().await;

        // completion_on
        if let Some(completion_on) = params.get("completion_on") {
            (*setting).completion_on = completion_on.as_bool().unwrap_or(setting.completion_on);
            self.client
                .log_message(
                    MessageType::INFO,
                    &format!("[ds-pinyin-lsp]: completion_on to {}!", completion_on),
                )
                .await;
        }

        // db_path
        if let Some(db_path) = params.get("db_path") {
            if let Some(db_path) = db_path.as_str() {
                if !db_path.is_empty() {
                    match setting.db_path {
                        Some(ref old_db_path) if old_db_path == db_path => {
                            self.client
                                .log_message(
                                    MessageType::INFO,
                                    "[ds-pinyin-lsp]: ignore same db_path!",
                                )
                                .await;
                        }
                        _ => {
                            // cache setting
                            (*setting).db_path = Some(db_path.to_string());

                            // open db connection
                            let conn = Connection::open(db_path);
                            if let Ok(conn) = conn {
                                let mut mutex = self.conn.lock().await;
                                *mutex = Some(conn);
                                self.client
                                    .log_message(
                                        MessageType::INFO,
                                        &format!("[ds-pinyin-lsp]: db connection to {}!", db_path),
                                    )
                                    .await;
                            } else if let Err(err) = conn {
                                self.client
                                    .show_message(
                                        MessageType::ERROR,
                                        &format!(
                                            "[ds-pinyin-lsp]: open database: {} error: {}",
                                            db_path, err
                                        ),
                                    )
                                    .await;
                            }
                        }
                    }
                } else {
                    // db_path empty
                    self.client
                        .show_message(
                            MessageType::ERROR,
                            "[ds-pinyin-lsp]: db_path is empty string!",
                        )
                        .await;
                }
            } else {
                // invalid db_path
                self.client
                    .show_message(
                        MessageType::ERROR,
                        "[ds-pinyin-lsp]: db_path must be string!",
                    )
                    .await;
            }
        }

        // check db_path
        if setting.db_path.is_none() {
            self.client
                .show_message(MessageType::ERROR, "[ds-pinyin-lsp]: db_path is missing!")
                .await;
        }

        // show_symbols
        if let Some(show_symbols) = params.get("show_symbols") {
            (*setting).show_symbols = show_symbols.as_bool().unwrap_or(setting.show_symbols);
            self.client
                .log_message(
                    MessageType::INFO,
                    &format!("[ds-pinyin-lsp]: show_symbols to {}!", show_symbols),
                )
                .await;
        }

        // match_as_same_as_input
        if let Some(match_as_same_as_input) = params.get("match_as_same_as_input") {
            (*setting).match_as_same_as_input = match_as_same_as_input
                .as_bool()
                .unwrap_or(setting.match_as_same_as_input);
            self.client
                .log_message(
                    MessageType::INFO,
                    &format!(
                        "[ds-pinyin-lsp]: match_as_same_as_input to {}!",
                        match_as_same_as_input
                    ),
                )
                .await;
        }

        // match_long_input
        if let Some(match_long_input) = params.get("match_long_input") {
            (*setting).match_long_input = match_long_input
                .as_bool()
                .unwrap_or(setting.match_long_input);
            self.client
                .log_message(
                    MessageType::INFO,
                    &format!("[ds-pinyin-lsp]: match_long_input to {}!", match_long_input),
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
    .custom_method("$/turn/completion", Backend::turn_completion)
    .finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}
