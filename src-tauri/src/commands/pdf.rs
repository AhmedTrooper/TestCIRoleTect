use tauri::{command, State, AppHandle, Manager};
use std::path::PathBuf;
use crate::{ai, AppState};
use tectonic::status::StatusBackend;
use tectonic::status::MessageKind;
use std::fmt::Arguments;

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
            // Also push to stdout for tauri dev logs
            eprintln!("Tectonic Error: {} - Detail: {}", msg, e);
        } else if kind == MessageKind::Error {
            eprintln!("Tectonic Error: {}", msg);
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

#[command]
pub async fn refine_latex_with_ai(
    state: State<'_, AppState>,
    provider: String,
    model: String,
    api_key: String,
    current_latex: String,
    instruction: String,
) -> Result<String, String> {
    let custom_base_url = crate::commands::settings::get_custom_base_url(&state, &provider).await;
    ai::refine_technical_content(&provider, &model, &api_key, custom_base_url.as_deref(), &current_latex, &instruction, "LaTeX").await
}

#[command]
pub async fn fix_latex_with_ai(
    state: State<'_, AppState>,
    provider: String,
    model: String,
    api_key: String,
    broken_latex: String,
    error_logs: String,
) -> Result<String, String> {
    let custom_base_url = crate::commands::settings::get_custom_base_url(&state, &provider).await;
    ai::fix_technical_errors(&provider, &model, &api_key, custom_base_url.as_deref(), &broken_latex, &error_logs, "LaTeX").await
}

#[command]
pub async fn refine_diagram_with_ai(
    state: State<'_, AppState>,
    provider: String,
    model: String,
    api_key: String,
    current_code: String,
    instruction: String,
    content_type: String,
) -> Result<String, String> {
    let custom_base_url = crate::commands::settings::get_custom_base_url(&state, &provider).await;
    ai::refine_technical_content(&provider, &model, &api_key, custom_base_url.as_deref(), &current_code, &instruction, &content_type).await
}

#[command]
pub async fn fix_diagram_with_ai(
    state: State<'_, AppState>,
    provider: String,
    model: String,
    api_key: String,
    broken_code: String,
    error_logs: String,
    content_type: String,
) -> Result<String, String> {
    let custom_base_url = crate::commands::settings::get_custom_base_url(&state, &provider).await;
    ai::fix_technical_errors(&provider, &model, &api_key, custom_base_url.as_deref(), &broken_code, &error_logs, &content_type).await
}

#[command]
pub async fn compile_resume_to_pdf(app_handle: AppHandle, latex_code: String, filename: Option<String>) -> Result<Vec<u8>, String> {
    let docs_dir = app_handle.path().document_dir().map_err(|e| format!("Failed to get documents dir: {}", e))?;
    let roletect_dir = docs_dir.join("RoleTect");
    if !roletect_dir.exists() {
        std::fs::create_dir_all(&roletect_dir).map_err(|e| format!("Failed to create RoleTect dir: {}", e))?;
    }
    let output_name = filename.unwrap_or_else(|| "output.pdf".to_string());
    let output_pdf_path = roletect_dir.join(output_name);

    tokio::task::spawn_blocking(move || {
        let thread_handle = std::thread::Builder::new()
            .name("tectonic-compiler".into())
            .stack_size(100 * 1024 * 1024)
            .spawn(move || {
                let mut status = CapturingStatusBackend::new();
                
                let config_loader = tectonic::config::PersistentConfig::default();
                let bundle = config_loader
                    .default_bundle(false)
                    .map_err(|e| format!("Failed to load Tectonic bundle: {}", e))?;

                let format_cache_path = config_loader
                    .format_cache_path()
                    .map_err(|e| format!("Failed to get format cache path: {}", e))?;

                let mut sb = tectonic::driver::ProcessingSessionBuilder::default();
                let temp_output_dir = std::env::temp_dir().join(format!("roletect-{}", nanoid::nanoid!()));
                std::fs::create_dir_all(&temp_output_dir).map_err(|e| format!("Failed to create temp output dir: {}", e))?;

                sb.bundle(bundle)
                    .primary_input_buffer(latex_code.as_bytes())
                    .tex_input_name("texput")
                    .filesystem_root(std::env::temp_dir()) // Use temp dir for intermediate files
                    .output_dir(&temp_output_dir)
                    .format_cache_path(format_cache_path)
                    .format_name("latex")
                    .output_format(tectonic::driver::OutputFormat::Pdf)
                    .build_date(std::time::SystemTime::now());

                let mut sess = sb.create(&mut status)
                    .map_err(|e| format!("Failed to create Tectonic session: {}\n\nLogs:\n{}", e, status.logs))?;

                sess.run(&mut status)
                    .map_err(|e| format!("Compilation failed: {}\n\nLogs:\n{}", e, status.logs))?;

                let temp_pdf_path = temp_output_dir.join("texput.pdf");
                if temp_pdf_path.exists() {
                    let pdf_data = std::fs::read(&temp_pdf_path).map_err(|e| format!("Failed to read generated PDF: {}", e))?;
                    
                    // Copy it to Documents/RoleTect/output.pdf
                    let _ = std::fs::write(&output_pdf_path, &pdf_data);

                    // Clean up temp dir
                    let _ = std::fs::remove_dir_all(&temp_output_dir);

                    Ok(pdf_data)
                } else {
                    let _ = std::fs::remove_dir_all(&temp_output_dir);
                    Err(format!("Compilation appeared successful, but PDF was not found at {:?}\n\nLogs:\n{}", temp_pdf_path, status.logs))
                }
            })
            .map_err(|e| format!("Failed to spawn compiler thread: {}", e))?;

        thread_handle
            .join()
            .map_err(|_| "Compiler thread panicked".to_string())?
    })
    .await
    .map_err(|e| format!("Blocking task failed: {}", e))?
}

#[command]
pub async fn compile_workspace_to_pdf(app_handle: AppHandle, workspace_dir: String, main_file_name: String, filename: Option<String>) -> Result<Vec<u8>, String> {
    let docs_dir = app_handle.path().document_dir().map_err(|e| format!("Failed to get documents dir: {}", e))?;
    let roletect_dir = docs_dir.join("RoleTect");
    if !roletect_dir.exists() {
        std::fs::create_dir_all(&roletect_dir).map_err(|e| format!("Failed to create RoleTect dir: {}", e))?;
    }
    let output_name = filename.unwrap_or_else(|| "output.pdf".to_string());
    let output_pdf_path = roletect_dir.join(output_name);

    let workspace_path = PathBuf::from(&workspace_dir);
    
    if !workspace_path.is_dir() {
        return Err(format!("Workspace path '{}' is not a valid directory.", workspace_dir));
    }

    tokio::task::spawn_blocking(move || {
        let thread_handle = std::thread::Builder::new()
            .name("tectonic-workspace-compiler".into())
            .stack_size(100 * 1024 * 1024)
            .spawn(move || {
                let mut status = CapturingStatusBackend::new();
                let workspace_path = PathBuf::from(&workspace_dir);
                
                let config_loader = tectonic::config::PersistentConfig::default();
                let bundle = config_loader
                    .default_bundle(false)
                    .map_err(|e| format!("Failed to load Tectonic bundle: {}", e))?;

                let format_cache_path = config_loader
                    .format_cache_path()
                    .map_err(|e| format!("Failed to get format cache path: {}", e))?;

                // Determine the absolute path to the main file
                let main_file_path = workspace_path.join(&main_file_name);
                if !main_file_path.is_file() {
                    return Err(format!("Main TeX file '{}' not found in workspace.", main_file_name));
                }


                let mut sb = tectonic::driver::ProcessingSessionBuilder::default();
                let temp_output_dir = std::env::temp_dir().join(format!("roletect-{}", nanoid::nanoid!()));
                std::fs::create_dir_all(&temp_output_dir).map_err(|e| format!("Failed to create temp output dir: {}", e))?;

                sb.bundle(bundle)
                    .primary_input_path(&main_file_path)
                    .tex_input_name("texput.tex")
                    .filesystem_root(&workspace_path)
                    .output_dir(&temp_output_dir) // Use temp dir for ALL outputs
                    .format_cache_path(format_cache_path)
                    .format_name("latex")
                    .output_format(tectonic::driver::OutputFormat::Pdf);

                let mut sess = sb.create(&mut status)
                    .map_err(|e| format!("Failed to create Tectonic session: {}\n\nLogs:\n{}", e, status.logs))?;

                sess.run(&mut status)
                    .map_err(|e| format!("Compilation failed: {}\n\nLogs:\n{}", e, status.logs))?;

                // The PDF will be named texput.pdf in the temp_output_dir
                let temp_pdf_path = temp_output_dir.join("texput.pdf");
                
                if temp_pdf_path.exists() {
                    let pdf_data = std::fs::read(&temp_pdf_path).map_err(|e| format!("Failed to read generated PDF: {}", e))?;
                    
                    // Copy it to the same directory as the compiling source file
                    let mut final_pdf_path = workspace_path.join(&main_file_name);
                    final_pdf_path.set_extension("pdf");
                    let _ = std::fs::write(&final_pdf_path, &pdf_data);

                    // ALSO copy to Documents/RoleTect/output.pdf
                    let _ = std::fs::write(&output_pdf_path, &pdf_data);

                    // Clean up temp dir
                    let _ = std::fs::remove_dir_all(&temp_output_dir);

                    Ok(pdf_data)
                } else {
                    let _ = std::fs::remove_dir_all(&temp_output_dir);
                    Err(format!("Compilation appeared successful, but PDF was not found at {:?}\n\nLogs:\n{}", temp_pdf_path, status.logs))
                }
            })
            .map_err(|e| format!("Failed to spawn compiler thread: {}", e))?;

        thread_handle
            .join()
            .map_err(|_| "The compiler thread panicked.".to_string())?
    })
    .await
    .map_err(|e| format!("The asynchronous task failed: {}", e))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use tectonic::status::{MessageKind, StatusBackend};

    const EXPECTED_STACK_SIZE: usize = 100 * 1024 * 1024; // 100 MB

    #[test]
    fn capturing_backend_records_errors() {
        let mut backend = CapturingStatusBackend::new();
        backend.report(MessageKind::Error, format_args!("missing \\begin{{document}}"), None);
        assert!(backend.logs.contains("error: missing \\begin{document}"));
    }

    #[test]
    fn capturing_backend_records_warnings() {
        let mut backend = CapturingStatusBackend::new();
        backend.report(MessageKind::Warning, format_args!("overfull hbox"), None);
        assert!(backend.logs.contains("warning: overfull hbox"));
    }

    #[test]
    fn capturing_backend_records_notes() {
        let mut backend = CapturingStatusBackend::new();
        backend.report(MessageKind::Note, format_args!("output written"), None);
        assert!(backend.logs.contains("note: output written"));
    }

    #[test]
    fn capturing_backend_dumps_error_logs() {
        let mut backend = CapturingStatusBackend::new();
        backend.dump_error_logs(b"! LaTeX Error: File not found.\n");
        assert!(backend.logs.contains("--- Underlying Error Logs ---"));
        assert!(backend.logs.contains("! LaTeX Error: File not found."));
    }

    #[test]
    fn compiler_thread_can_spawn_with_100mb_stack() {
        let handle = std::thread::Builder::new()
            .name("stack-size-test".into())
            .stack_size(EXPECTED_STACK_SIZE)
            .spawn(|| {
                // Allocate a modest chunk on the stack to prove the space is available
                let _buf = [0u8; 1024 * 1024]; // 1 MB on stack
                42
            })
            .expect("Failed to spawn thread with 100MB stack");

        let result = handle.join().expect("Thread panicked");
        assert_eq!(result, 42);
    }
}
