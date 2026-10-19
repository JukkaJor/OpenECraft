//! OpenECraft – an open source electrical CAE editor (schematics, device tags,
//! cross references, reports) inspired by EPLAN Electric P8.
//!
//! Runs natively on Windows, Linux and macOS and in the browser (WebAssembly).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
#[cfg(test)]
mod conventions;
mod editor;
mod geom;
#[cfg(test)]
mod glyphs;
mod history;
mod index;
mod io;
mod model;
mod netlist;
mod ops;
mod parts;
mod parts_ui;
mod render;
mod reports;
mod sample;
mod svg;
mod symbols;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("OpenECraft")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native("OpenECraft", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast;
    eframe::WebLogger::init(log::LevelFilter::Info).ok();
    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().expect("no window").document().expect("no document");
        let canvas = document
            .get_element_by_id("openecraft_canvas")
            .expect("canvas missing")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("not a canvas");
        let result = eframe::WebRunner::new()
            .start(canvas, eframe::WebOptions::default(), Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
            .await;
        if let Some(loading) = document.get_element_by_id("loading") {
            match result {
                Ok(()) => loading.remove(),
                Err(e) => {
                    loading.set_inner_html("<p>OpenECraft failed to start. See the browser console for details.</p>");
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
