use axum::{Json, response::IntoResponse, http::{StatusCode, header}};
use tectonic::status::StatusBackend;
use tectonic::status::MessageKind;
use std::fmt::Arguments;
use crate::ai;

pub struct CapturingStatusBackend {
    pub logs: String,
}

impl CapturingStatusBackend {
    pub fn new() -> Self {
        Self { logs: String::new() }
    }
}

impl StatusBackend for CapturingStatusBackend {
    fn report(&mut self, kind: MessageKind, args: Arguments, err: Option<&anyhow::Error>) {
        let prefix = match kind {
            MessageKind::Error => "error: ",
            MessageKind::Warning => "warning: ",
            MessageKind::Note => "note: ",
        };
        let msg = format!("{}", args);
        self.logs.push_str(prefix);
        self.logs.push_str(&msg);
        
        if let Some(e) = err {
            self.logs.push_str(&format!(" (error detail: {})", e));
        }
        
        self.logs.push('\n');
    }

    fn dump_error_logs(&mut self, logs: &[u8]) {
        if let Ok(s) = std::str::from_utf8(logs) {
            self.logs.push_str("--- Underlying Error Logs ---\n");
            self.logs.push_str(s);
            self.logs.push('\n');
        }
    }
}

pub async fn refine_latex_with_ai(
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<String>, (StatusCode, String)> {
    let provider = payload["provider"].as_str().ok_or((StatusCode::BAD_REQUEST, "provider missing".to_string()))?;
    let model = payload["model"].as_str().ok_or((StatusCode::BAD_REQUEST, "model missing".to_string()))?;
    let api_key = payload["apiKey"].as_str().ok_or((StatusCode::BAD_REQUEST, "apiKey missing".to_string()))?;
    let current_latex = payload["currentLatex"].as_str().ok_or((StatusCode::BAD_REQUEST, "currentLatex missing".to_string()))?;
    let instruction = payload["instruction"].as_str().ok_or((StatusCode::BAD_REQUEST, "instruction missing".to_string()))?;

    ai::refine_technical_content(provider, model, api_key, current_latex, instruction, "LaTeX")
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))
}

pub async fn compile_latex(
    Json(payload): Json<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let latex_code = payload["latexContent"].as_str().ok_or((StatusCode::BAD_REQUEST, "latexContent missing".to_string()))?;
    let latex_code_owned = latex_code.to_string();

    let pdf_result = tokio::task::spawn_blocking(move || {
        let thread_handle = std::thread::Builder::new()
            .name("tectonic-compiler".into())
            .stack_size(10 * 1024 * 1024)
            .spawn(move || {
                let mut status = CapturingStatusBackend::new();
                
                let config_loader = tectonic::config::PersistentConfig::default();
                let bundle = config_loader
                    .default_bundle(false)
                    .map_err(|e| format!("Failed to load Tectonic bundle: {}", e))?;

                let mut sb = tectonic::driver::ProcessingSessionBuilder::default();
                sb.bundle(bundle)
                    .primary_input_buffer(latex_code_owned.as_bytes())
                    .tex_input_name("texput")
                    .filesystem_root(std::env::temp_dir())
                    .format_name("latex")
                    .output_format(tectonic::driver::OutputFormat::Pdf)
                    .build_date(std::time::SystemTime::now());

                let mut sess = sb.create(&mut status)
                    .map_err(|e| format!("Failed to create Tectonic session: {}\n\nLogs:\n{}", e, status.logs))?;

                sess.run(&mut status)
                    .map_err(|e| format!("Compilation failed: {}\n\nLogs:\n{}", e, status.logs))?;

                let out_data = sess.into_file_data();
                
                out_data.get("texput.pdf")
                    .cloned()
                    .ok_or_else(|| format!("Compilation appeared successful, but 'texput.pdf' was not generated.\n\nLogs:\n{}", status.logs))
                    .map(|f| f.data)
            })
            .map_err(|e| format!("Failed to spawn compiler thread: {}", e))?;

        thread_handle
            .join()
            .map_err(|_| "Compiler thread panicked".to_string())?
    })
    .await;

    match pdf_result {
        Ok(Ok(pdf_data)) => {
            Ok((
                [(header::CONTENT_TYPE, "application/pdf"), (header::CONTENT_DISPOSITION, "inline; filename=\"preview.pdf\"")],
                pdf_data,
            ))
        }
        Ok(Err(e)) => {
            eprintln!("LaTeX Compilation Error:\n{}", e);
            Err((StatusCode::BAD_REQUEST, e)) // Return 400 for LaTeX errors
        }
        Err(e) => {
            eprintln!("Server Task Error: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Internal task error: {}", e)))
        }
    }
}
