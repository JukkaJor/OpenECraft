//! OpenECraft – an open source electrical CAE editor (schematics, device tags,
//! cross references, reports) inspired by EPLAN Electric P8.
//!
//! Runs natively on Windows, Linux and macOS and in the browser (WebAssembly).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
#[cfg(test)]
mod conventions;
mod editor;
#[cfg(test)]
mod glyphs;
mod io;
mod panel_ui;
mod parts_ui;
mod wiring_ui;

use openecraft_core::{
    climate, config, document, dxf, dxf_import, export3d, fluid, forms, frame, geom, harness, history, index, macros, marking, model, netlist, ops, page_templates, panel, parts, pdf, plc, plugs, potentials, process, production, render, reports,
    revision, sample, strips, structure, svg, symbols, symlib, tables, translation, wirenum, wiring,
};

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
    // `--wiring` opens the wiring assistant (touch screens in the workshop).
    let wiring = std::env::args().any(|a| a == "--wiring");
    eframe::run_native("OpenECraft", options, Box::new(move |cc| Ok(Box::new(app::App::new(cc).start_wiring(wiring)))))
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast;
    eframe::WebLogger::init(log::LevelFilter::Info).ok();
    if let Some(lang) = web_sys::window().and_then(|w| w.navigator().language()).and_then(|code| openecraft_i18n::Lang::from_code(&code)) {
        openecraft_i18n::set_lang(lang);
    }
    // `…/#wiring` opens the wiring assistant (tablets in the workshop).
    let wiring = web_sys::window().and_then(|w| w.location().hash().ok()).is_some_and(|h| h == "#wiring");
    wasm_bindgen_futures::spawn_local(async move {
        let document = web_sys::window().expect("no window").document().expect("no document");
        let canvas = document
            .get_element_by_id("openecraft_canvas")
            .expect("canvas missing")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("not a canvas");
        let result = eframe::WebRunner::new()
            .start(canvas, eframe::WebOptions::default(), Box::new(move |cc| Ok(Box::new(app::App::new(cc).start_wiring(wiring)))))
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
