//! Platform specific file handling: native file dialogs on desktop, file picker and
//! downloads in the browser.

use std::sync::{Arc, Mutex};

pub struct Loaded {
    /// Native: full path. Web: file name.
    pub name: String,
    pub content: String,
}

#[derive(Default, Clone)]
pub struct FileIo {
    inbox: Arc<Mutex<Vec<Loaded>>>,
}

pub const PROJECT_EXT: &str = "oec";

impl FileIo {
    /// Takes a file that finished loading (the web picker is asynchronous).
    pub fn poll(&self) -> Option<Loaded> {
        self.inbox.lock().ok()?.pop()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_project(&self, _ctx: &egui::Context) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("OpenECraft project", &[PROJECT_EXT, "json"])
            .pick_file()
        else {
            return;
        };
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                if let Ok(mut inbox) = self.inbox.lock() {
                    inbox.push(Loaded { name: path.display().to_string(), content });
                }
            }
            Err(e) => log::error!("Could not read {}: {e}", path.display()),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn open_project(&self, ctx: &egui::Context) {
        let inbox = self.inbox.clone();
        let ctx = ctx.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("OpenECraft project", &[PROJECT_EXT, "json"])
                .pick_file()
                .await
            else {
                return;
            };
            let bytes = file.read().await;
            let content = String::from_utf8_lossy(&bytes).to_string();
            if let Ok(mut inbox) = inbox.lock() {
                inbox.push(Loaded { name: file.file_name(), content });
            }
            ctx.request_repaint();
        });
    }

    /// Asks for a location and writes the file. Returns the chosen path / name.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_as(&self, suggested: &str, filter: (&str, &str), content: &str) -> Result<Option<String>, String> {
        let Some(path) = rfd::FileDialog::new().add_filter(filter.0, &[filter.1]).set_file_name(suggested).save_file()
        else {
            return Ok(None);
        };
        std::fs::write(&path, content).map_err(|e| e.to_string())?;
        Ok(Some(path.display().to_string()))
    }

    /// In the browser "save as" is a download.
    #[cfg(target_arch = "wasm32")]
    pub fn save_as(&self, suggested: &str, filter: (&str, &str), content: &str) -> Result<Option<String>, String> {
        let mime = match filter.1 {
            "svg" => "image/svg+xml",
            "html" => "text/html",
            "csv" => "text/csv",
            _ => "application/json",
        };
        download(suggested, mime, content).map_err(|e| format!("{e:?}"))?;
        Ok(Some(suggested.to_string()))
    }

    /// Writes to a known location (native) or downloads again (web).
    pub fn save_to(&self, path: &str, content: &str) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::fs::write(path, content).map_err(|e| e.to_string())
        }
        #[cfg(target_arch = "wasm32")]
        {
            download(path, "application/json", content).map_err(|e| format!("{e:?}"))
        }
    }

    pub fn can_save_in_place() -> bool {
        cfg!(not(target_arch = "wasm32"))
    }
}

#[cfg(target_arch = "wasm32")]
fn download(name: &str, mime: &str, content: &str) -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast;
    let parts = js_sys::Array::new();
    parts.push(&wasm_bindgen::JsValue::from_str(content));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(mime);
    let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &opts)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;
    let document = web_sys::window().and_then(|w| w.document()).ok_or("no document")?;
    let a: web_sys::HtmlAnchorElement = document.create_element("a")?.dyn_into()?;
    a.set_href(&url);
    a.set_download(name);
    a.click();
    web_sys::Url::revoke_object_url(&url)?;
    Ok(())
}

/// File name without directories, for window titles.
pub fn short_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}
