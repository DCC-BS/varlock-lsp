mod catalog;
mod completion;
mod diagnostics;
mod hover;
mod parser;
mod server;

use tower_lsp::LspService;

#[tokio::main]
async fn main() {
    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());

    let (service, socket) = LspService::new(|client| server::EnvSpecLsp::new(client));

    tower_lsp::Server::new(stdin, stdout, socket)
        .serve(service)
        .await;
}
