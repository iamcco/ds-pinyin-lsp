use dashmap::DashMap;
use ds_pinyin_lsp::sqlite::query_dict;
use ds_pinyin_lsp::types::Setting;
use ds_pinyin_lsp::utils::{
    get_current_line, get_pinyin, long_suggests_to_completion_item, query_long_sentence,
    suggests_to_completion_item, symbols_to_completion_item,
};
use lsp_document::{apply_change, IndexedText, TextAdapter};
use regex::Regex;
use rusqlite::Connection;
use serde_json::Value;
use tokio::sync::{Mutex, MutexGuard};
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
    chinese_symbols: String,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(initialization_options) = params.initialization_options {
            self.change_configuration(&initialization_options).await;
        } else {
            self.error("[ds-pinyin-lsp]: initialization_options is missing, it must include db_path setting!").await;
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
        self.info(&format!("Close file: {}", &uri)).await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        // check completion on/off
        let setting = self.setting.lock().await;
        if !setting.completion_on {
            return Ok(Some(CompletionResponse::Array(vec![])));
        }

        let uri = params.text_document_position.text_document.uri.to_string();
        let document = self.documents.get(&uri);
        if let None = document {
            return Ok(Some(CompletionResponse::Array(vec![])));
        }

        let position = params.text_document_position.position;
        let (backward_line, forward_line) = get_current_line(
            document.as_ref().unwrap(), // document will never be None here
            &position,
        )
        .unwrap_or(("", ""));

        if backward_line.is_empty() {
            return Ok(Some(CompletionResponse::Array(vec![])));
        }

        let pinyin = get_pinyin(backward_line).unwrap_or(String::new());

        if pinyin.is_empty() {
            if setting.show_symbols {
                // check symbol
                if let Some(last_char) = backward_line.chars().last() {
                    if let Some(symbols) = self.symbols.get(&last_char) {
                        // show_symbols_by_n_times
                        let times = setting.show_symbols_by_n_times;
                        if times > 0
                            && backward_line.len() as u64 >= times
                            && Regex::new(&format!(
                                "{}$",
                                regex::escape(&String::from(last_char).repeat(times as usize))
                            ))
                            .unwrap()
                            .is_match(backward_line)
                        {
                            return Ok(Some(CompletionResponse::List(CompletionList {
                                is_incomplete: true,
                                items: symbols_to_completion_item(
                                    last_char, symbols, position, times,
                                ),
                            })));
                        }
                        // show_symbols_only_follow_by_hanzi
                        if !setting.show_symbols_only_follow_by_hanzi
                            || (backward_line.len() > 1
                                && Regex::new(r"\p{Han}$")
                                    .unwrap()
                                    .is_match(&backward_line[..backward_line.len() - 1]))
                        {
                            return Ok(Some(CompletionResponse::List(CompletionList {
                                is_incomplete: true,
                                items: symbols_to_completion_item(last_char, symbols, position, 1),
                            })));
                        }
                    }
                }
            }

            // return for empty pinyin
            return Ok(Some(CompletionResponse::Array(vec![])));
        }

        // 触发模式
        let trigger_completion = !setting.completion_trigger_characters.is_empty()
            && Regex::new(&format!(
                "{}[a-zA-Z]+$",
                regex::escape(&setting.completion_trigger_characters)
            ))
            .unwrap()
            .is_match(backward_line);

        // 环绕模式
        let around_completion = Regex::new(&format!(
            r"(\p{{Han}}|{})(\s*\w+\s+)*[a-zA-Z]+$",
            self.chinese_symbols
        ))
        .unwrap()
        .is_match(backward_line)
            || Regex::new(&format!(
                r"^(\s*\w+\s*)*(\p{{Han}}|{})",
                self.chinese_symbols
            ))
            .unwrap()
            .is_match(forward_line);

        // 开启环绕补全模式，但是：
        // - 不符合环绕模式
        // - 不符合触发模式
        if setting.completion_around_mode && !around_completion && !trigger_completion {
            return Ok(Some(CompletionResponse::Array(vec![])));
        }

        // pinyin range
        let range = Range::new(
            Position {
                line: position.line,
                character: position.character
                    - (if trigger_completion {
                        pinyin.len() + setting.completion_trigger_characters.len()
                    } else {
                        pinyin.len()
                    }) as u32,
            },
            position,
        );

        if let Some(ref conn) = *self.conn.lock().await {
            // dict search match
            if let Ok(suggests) = query_dict(
                conn,
                &pinyin,
                setting.max_suggest,
                setting.match_as_same_as_input,
            ) {
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

        self.info(&format!(
            "[ds-pinyin-lsp]: completion_on: {}",
            setting.completion_on
        ))
        .await;
    }

    async fn change_configuration(&self, params: &Value) {
        let mut setting = self.setting.lock().await;

        for option_key in [
            "db_path",
            "completion_on",
            "completion_around_mode",
            "completion_trigger_characters",
            "show_symbols",
            "show_symbols_only_follow_by_hanzi",
            "show_symbols_by_n_times",
            "match_as_same_as_input",
            "match_long_input",
            "max_suggest",
        ] {
            if let Some(option) = params.get(option_key) {
                match option_key {
                    "db_path" => {
                        if let Some(db_path) = option.as_str() {
                            self.update_db_path(&mut setting, db_path).await;
                        } else {
                            // invalid db_path
                            self.error("[ds-pinyin-lsp]: db_path must be string!").await;
                        }
                    }
                    "completion_on" => {
                        (*setting).completion_on =
                            option.as_bool().unwrap_or(setting.completion_on);
                    }
                    "completion_around_mode" => {
                        (*setting).completion_around_mode =
                            option.as_bool().unwrap_or(setting.completion_around_mode);
                    }
                    "completion_trigger_characters" => {
                        (*setting).completion_trigger_characters =
                            option.as_str().unwrap_or("").to_string();
                    }
                    "show_symbols" => {
                        (*setting).show_symbols = option.as_bool().unwrap_or(setting.show_symbols);
                    }
                    "show_symbols_only_follow_by_hanzi" => {
                        (*setting).show_symbols_only_follow_by_hanzi = option
                            .as_bool()
                            .unwrap_or(setting.show_symbols_only_follow_by_hanzi);
                    }
                    "show_symbols_by_n_times" => {
                        (*setting).show_symbols_by_n_times =
                            option.as_u64().unwrap_or(setting.show_symbols_by_n_times);
                    }
                    "match_as_same_as_input" => {
                        (*setting).match_as_same_as_input =
                            option.as_bool().unwrap_or(setting.match_as_same_as_input);
                    }
                    "match_long_input" => {
                        (*setting).match_long_input =
                            option.as_bool().unwrap_or(setting.match_long_input);
                    }
                    "max_suggest" => {
                        (*setting).max_suggest = option.as_u64().unwrap_or(setting.max_suggest);
                    }
                    _ => {}
                }

                self.info(&format!("[ds-pinyin-lsp]: {} to {}!", option_key, option))
                    .await
            }
        }

        // check db_path
        if setting.db_path.is_empty() {
            self.error("[ds-pinyin-lsp]: db_path is missing!").await;
        }
    }

    async fn update_db_path<'a>(&self, setting: &mut MutexGuard<'a, Setting>, db_path: &str) {
        if db_path.is_empty() {
            self.error("[ds-pinyin-lsp]: db_path is empty string!")
                .await;
            return;
        }
        if setting.db_path == db_path {
            self.info("[ds-pinyin-lsp]: ignore same db_path!").await;
            return;
        }
        match Connection::open(db_path) {
            Ok(conn) => {
                // cache setting
                (*setting).db_path = db_path.to_string();
                // connection
                let mut mutex = self.conn.lock().await;
                *mutex = Some(conn);
                self.info(&format!("[ds-pinyin-lsp]: db connection to {}!", db_path))
                    .await;
            }
            Err(err) => {
                self.error(&format!(
                    "[ds-pinyin-lsp]: open database: {} error: {}",
                    db_path, err
                ))
                .await;
            }
        }
    }

    async fn info(&self, message: &str) {
        self.client.log_message(MessageType::INFO, message).await;
    }

    async fn error(&self, message: &str) {
        self.client.log_message(MessageType::ERROR, message).await;
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
        chinese_symbols: String::from(
            "。|·|……|～|、|，|；|：|？|！|“|”|‘|’|（|）|——|《|》|【|】|¥",
        ),
    })
    .custom_method("$/turn/completion", Backend::turn_completion)
    .finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}
