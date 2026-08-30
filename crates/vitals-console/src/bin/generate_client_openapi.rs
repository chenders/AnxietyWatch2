//! Emit the deterministic Stage 7 client `OpenAPI` document to stdout.

use std::io::{self, Write};
use utoipa::OpenApi;
use vitals_console::client_api::ClientApiDoc;

fn main() {
    let document = match serde_json::to_string_pretty(&ClientApiDoc::openapi()) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("generate-client-openapi: serialization failed: {error}");
            std::process::exit(1);
        }
    };
    if let Err(error) = writeln!(io::stdout(), "{document}") {
        eprintln!("generate-client-openapi: stdout write failed: {error}");
        std::process::exit(1);
    }
}
