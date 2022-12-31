use dashmap::DashMap;
use ds_pinyin_lsp::{lsp::Backend, types::Setting};
use tokio::sync::Mutex;
use tower_lsp::{LspService, Server};

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
