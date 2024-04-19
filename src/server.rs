use crate::lexer::Lexer;
use anyhow::Result;
use log::info;
use lsp_server::{Connection, Message, Notification};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams,
    DidOpenTextDocumentParams, PublishDiagnosticsParams, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, Url,
};
use streaming_iterator::StreamingIterator;
use zspell::Dictionary;

pub struct Server {
    dict: Dictionary,
    did_shutdown: bool,
    did_exit: bool,
}

impl Server {
    pub fn new(dict: Dictionary) -> Result<Self> {
        Ok(Self {
            dict,
            did_shutdown: false,
            did_exit: false,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let (connection, iothreads) = Connection::stdio();
        _ = iothreads;

        let mut server_capabilities = ServerCapabilities::default();
        server_capabilities.text_document_sync =
            Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL));
        let _ = connection
            .initialize(serde_json::to_value(server_capabilities)?)?;

        while !self.did_exit {
            let message = connection.receiver.recv()?;

            match message {
                Message::Notification(notification) => {
                    let response = self.handle_notification(notification)?;
                    if let Some(response) = response {
                        connection
                            .sender
                            .send(Message::Notification(response))?;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn handle_notification(
        &mut self,
        notification: Notification,
    ) -> Result<Option<Notification>> {
        info!(
            "received notification with method: {}",
            notification.method.as_str()
        );
        match notification.method.as_str() {
            "textDocument/didOpen" => {
                let params = serde_json::from_value::<DidOpenTextDocumentParams>(
                    notification.params,
                )?;
                let uri = params.text_document.uri;
                let text = params.text_document.text;
                self.make_diagnostics(uri, text.as_str())
            }
            "textDocument/didChange" => {
                let params = serde_json::from_value::<
                    DidChangeTextDocumentParams,
                >(notification.params)?;
                let uri = params.text_document.uri;
                let text = params.content_changes[0].text.as_str();
                self.make_diagnostics(uri, text)
            }
            "shutdown" => {
                // TODO: handle shutdown correctly
                self.did_shutdown = true;
                self.did_exit = true;
                Ok(None)
            }
            "exit" => Ok(None),
            _ => Ok(None),
        }
    }

    fn make_diagnostics(
        &mut self,
        uri: Url,
        text: &str,
    ) -> Result<Option<Notification>> {
        let mut lexer = match Lexer::new(text) {
            None => return Ok(None),
            Some(lexer) => lexer,
        };

        let mut params = PublishDiagnosticsParams {
            uri,
            diagnostics: Vec::new(),
            version: None,
        };

        while let Some(word) = lexer.next() {
            if !self.dict.check(word.text) {
                params.diagnostics.push(Diagnostic {
                    range: word.range,
                    message: "Incorrect spelling".to_string(),
                    severity: Some(DiagnosticSeverity::ERROR),
                    ..Default::default()
                });
            }
        }

        Ok(Some(Notification::new(
            "textDocument/publishDiagnostics".to_string(),
            params,
        )))
    }
}
