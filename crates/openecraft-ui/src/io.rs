//! Platform specific file handling: native file dialogs on desktop, file picker and
//! downloads in the browser. Generic over the application's kinds of files.

use std::sync::{Arc, Mutex};

/// What a file is opened for; provides the file dialog filter.
pub trait FileKind: Copy + Send + 'static {
    /// Filter name and extensions.
    fn filter(self) -> (&'static str, &'static [&'static str]);
}

pub struct Loaded<K> {
    pub purpose: K,
    /// Native: full path. Web: file name.
    pub name: String,
    /// Raw file content (binary STL files are not text).
    pub bytes: Vec<u8>,
}

impl<K> Loaded<K> {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }
}

pub struct FileIo<K> {
    inbox: Arc<Mutex<Vec<Loaded<K>>>>,
}

impl<K> Default for FileIo<K> {
    fn default() -> Self {
        FileIo { inbox: Arc::default() }
    }
}

impl<K> Clone for FileIo<K> {
    fn clone(&self) -> Self {
        FileIo { inbox: self.inbox.clone() }
    }
}

impl<K: FileKind> FileIo<K> {
    /// Takes a file that finished loading (the web picker is asynchronous).
    pub fn poll(&self) -> Option<Loaded<K>> {
        self.inbox.lock().ok()?.pop()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn open(&self, _ctx: &egui::Context, purpose: K) {
        let (name, ext) = purpose.filter();
        let Some(path) = rfd::FileDialog::new()
            .add_filter(name, ext)
            .pick_file()
        else {
            return;
        };
        match std::fs::read(&path) {
            Ok(bytes) => {
                if let Ok(mut inbox) = self.inbox.lock() {
                    inbox.push(Loaded { purpose, name: path.display().to_string(), bytes });
                }
            }
            Err(e) => log::error!("Could not read {}: {e}", path.display()),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn open(&self, ctx: &egui::Context, purpose: K) {
        let inbox = self.inbox.clone();
        let ctx = ctx.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let (name, ext) = purpose.filter();
            let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter(name, ext)
                .pick_file()
                .await
            else {
                return;
            };
            let bytes = file.read().await;
            if let Ok(mut inbox) = inbox.lock() {
                inbox.push(Loaded { purpose, name: file.file_name(), bytes });
            }
            ctx.request_repaint();
        });
    }
}

impl<K> FileIo<K> {
    /// Asks for a location and writes the file. Returns the chosen path / name.
    pub fn save_as(&self, suggested: &str, filter: (&str, &str), content: &str) -> Result<Option<String>, String> {
        self.save_bytes_as(suggested, filter, content.as_bytes())
    }

    /// Like [`FileIo::save_as`] for binary content.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_bytes_as(&self, suggested: &str, filter: (&str, &str), content: &[u8]) -> Result<Option<String>, String> {
        let Some(path) = rfd::FileDialog::new().add_filter(filter.0, &[filter.1]).set_file_name(suggested).save_file()
        else {
            return Ok(None);
        };
        std::fs::write(&path, content).map_err(|e| e.to_string())?;
        Ok(Some(path.display().to_string()))
    }

    /// In the browser "save as" is a download.
    #[cfg(target_arch = "wasm32")]
    pub fn save_bytes_as(&self, suggested: &str, filter: (&str, &str), content: &[u8]) -> Result<Option<String>, String> {
        download(suggested, mime_type(filter.1), content).map_err(|e| format!("{e:?}"))?;
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
            download(path, "application/json", content.as_bytes()).map_err(|e| format!("{e:?}"))
        }
    }

    pub fn can_save_in_place() -> bool {
        cfg!(not(target_arch = "wasm32"))
    }
}

/// MIME type of a download by file extension.
pub fn mime_type(ext: &str) -> &'static str {
    match ext {
        "svg" => "image/svg+xml",
        "html" => "text/html",
        "csv" => "text/csv",
        "pdf" => "application/pdf",
        "dxf" => "application/dxf",
        "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "png" | "jpg" | "jpeg" | "zip" => "application/octet-stream",
        _ => "application/json",
    }
}

#[cfg(target_arch = "wasm32")]
fn download(name: &str, mime: &str, content: &[u8]) -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast;
    let parts = js_sys::Array::new();
    parts.push(&js_sys::Uint8Array::from(content));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(mime);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts)?;
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

#[cfg(test)]
mod tests;
