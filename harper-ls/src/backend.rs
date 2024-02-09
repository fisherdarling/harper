use std::{collections::HashMap, sync::Arc};

use harper_core::{parsers::Markdown, Document, FullDictionary, LintSet, Linter, MergedDictionary};
use tokio::sync::Mutex;
use tower_lsp::{
    jsonrpc::Result,
    lsp_types::{
        notification::{PublishDiagnostics, ShowMessage},
        CodeAction, CodeActionOrCommand, CodeActionParams, CodeActionProviderCapability,
        CodeActionResponse, Diagnostic, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
        DidOpenTextDocumentParams, DidSaveTextDocumentParams, InitializeParams, InitializeResult,
        InitializedParams, MessageType, PublishDiagnosticsParams, Range, ServerCapabilities,
        ShowMessageParams, TextDocumentSyncCapability, TextDocumentSyncKind,
        TextDocumentSyncOptions, TextDocumentSyncSaveOptions, Url,
    },
    Client, LanguageServer,
};

use crate::{
    diagnostics::{lint_to_code_actions, lints_to_diagnostics},
    pos_conv::range_to_span,
    tree_sitter_parser::TreeSitterParser,
};

pub struct Backend {
    client: Client,
    global_dictionary: Arc<FullDictionary>,
    files: Mutex<HashMap<Url, Document>>,
    /// The identifiers extracted from each file by Tree-sitter.
    ident_dicts: Mutex<HashMap<Url, Arc<FullDictionary>>>,
}

impl Backend {
    async fn update_document_from_file(&self, url: &Url) {
        let Ok(content) = tokio::fs::read_to_string(url.path()).await else {
            // TODO: Proper error handling here.
            return;
        };
        self.update_document(url, &content).await;
    }

    async fn update_document(&self, url: &Url, text: &str) {
        let doc = if let Some(extension) = url.to_file_path().unwrap().extension() {
            if let Some(ts_parser) =
                TreeSitterParser::new_from_extension(&extension.to_string_lossy())
            {
                let doc = Document::new(text, Box::new(ts_parser.clone()));

                if let Some(new_dict) = ts_parser.create_ident_dict(doc.get_full_content()) {
                    let mut ident_dicts = self.ident_dicts.lock().await;
                    ident_dicts.insert(url.clone(), new_dict.into());
                }

                doc
            } else {
                Document::new(text, Box::new(Markdown))
            }
        } else {
            Document::new(text, Box::new(Markdown))
        };

        let mut files = self.files.lock().await;
        files.insert(url.clone(), doc);
    }

    async fn create_linter(&self, url: &Url) -> LintSet {
        let mut dictionary = MergedDictionary::new();
        dictionary.add_dictionary(self.global_dictionary.clone());

        if let Some(ident_dict) = self.ident_dicts.lock().await.get(url) {
            dictionary.add_dictionary(ident_dict.clone());
        };

        LintSet::new().with_standard(dictionary)
    }

    async fn generate_code_actions(&self, url: &Url, range: Range) -> Result<Vec<CodeAction>> {
        let files = self.files.lock().await;
        let Some(document) = files.get(url) else {
            return Ok(vec![]);
        };

        let mut linter = self.create_linter(url).await;
        let mut lints = linter.lint(document);
        lints.sort_by_key(|l| l.priority);

        let source_chars = document.get_full_content();

        // Find lints whose span overlaps with range
        let span = range_to_span(source_chars, range);

        let actions = lints
            .into_iter()
            .filter(|lint| lint.span.overlaps_with(span))
            .flat_map(|lint| lint_to_code_actions(&lint, url, source_chars).collect::<Vec<_>>())
            .collect();

        Ok(actions)
    }

    pub fn new(client: Client) -> Self {
        let dictionary = FullDictionary::create_from_curated();

        Self {
            client,
            global_dictionary: dictionary.into(),
            files: Mutex::new(HashMap::new()),
            ident_dicts: Mutex::new(HashMap::new()),
        }
    }

    async fn generate_diagnostics(&self, url: &Url) -> Vec<Diagnostic> {
        let files = self.files.lock().await;

        let Some(document) = files.get(url) else {
            return vec![];
        };

        let mut linter = self.create_linter(url).await;
        let lints = linter.lint(document);

        lints_to_diagnostics(document.get_full_content(), &lints)
    }

    async fn publish_diagnostics(&self, url: &Url) {
        let client = self.client.clone();

        tokio::spawn(async move {
            client
                .send_notification::<ShowMessage>(ShowMessageParams {
                    typ: MessageType::INFO,
                    message: "Linting...".to_string(),
                })
                .await
        });

        let diagnostics = self.generate_diagnostics(url).await;

        let result = PublishDiagnosticsParams {
            uri: url.clone(),
            diagnostics,
            version: None,
        };

        self.client
            .send_notification::<PublishDiagnostics>(result)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        will_save: None,
                        will_save_wait_until: None,
                        save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                    },
                )),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "File saved!")
            .await;

        self.update_document_from_file(&params.text_document.uri)
            .await;
        self.publish_diagnostics(&params.text_document.uri).await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "File opened!")
            .await;

        self.update_document_from_file(&params.text_document.uri)
            .await;

        self.publish_diagnostics(&params.text_document.uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let Some(last) = params.content_changes.last() else {
            return;
        };

        self.client
            .log_message(MessageType::INFO, "File changed!")
            .await;

        self.update_document(&params.text_document.uri, &last.text)
            .await;
        self.publish_diagnostics(&params.text_document.uri).await;
    }

    async fn did_close(&self, _params: DidCloseTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "File closed!")
            .await;
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let actions = self
            .generate_code_actions(&params.text_document.uri, params.range)
            .await?;

        self.client
            .log_message(MessageType::INFO, format!("{:?}", actions))
            .await;

        Ok(Some(
            actions
                .into_iter()
                .map(CodeActionOrCommand::CodeAction)
                .collect(),
        ))
    }
}
