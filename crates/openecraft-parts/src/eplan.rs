//! EPLAN parts XML (the parts management export of EPLAN Platform: `<part>` elements
//! with `P_ARTICLE_*` attributes, optionally with `<variant>` children): read into
//! parts and written for EPLAN. Multi-language texts (`de_DE@Schütz;en_US@Contactor;`)
//! take the English text, otherwise German, otherwise the first one. Product groups
//! are mapped by their exchange numbers ([`crate::productgroups`]).

use crate::productgroups::{self, Classification};
use crate::xml::{self, Element};
use crate::Part;
use std::fmt::Write;

/// The text looks like an EPLAN parts XML file.
pub fn is_eplan_xml(text: &str) -> bool {
    let t = text.trim_start_matches('\u{feff}').trim_start();
    t.starts_with('<') && t.contains("P_ARTICLE_PARTNR")
}

/// The text of a multi-language value in the preferred language.
pub fn language_text(value: &str) -> String {
    if !value.contains('@') {
        return value.trim().to_string();
    }
    let entries: Vec<(&str, &str)> = value.split(';').filter_map(|e| e.split_once('@')).map(|(l, t)| (l.trim(), t.trim())).filter(|(_, t)| !t.is_empty()).collect();
    for lang in ["en_US", "en_GB", "de_DE"] {
        if let Some((_, t)) = entries.iter().find(|(l, _)| l.eq_ignore_ascii_case(lang)) {
            return t.to_string();
        }
    }
    entries.first().map(|(_, t)| t.to_string()).unwrap_or_else(|| value.trim().to_string())
}

fn attr<'a>(el: &'a Element, name: &str) -> Option<&'a str> {
    el.attributes.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str()).filter(|v| !v.trim().is_empty())
}

/// A number written by EPLAN (`45`, `45.5`, `45,5`); 0 if there is none.
fn num(v: Option<&str>) -> f32 {
    v.map(|s| s.trim().replace(',', ".")).and_then(|s| s.parse().ok()).unwrap_or(0.0)
}

fn part_of(el: &Element, variant: Option<&Element>) -> Option<Part> {
    // Values of the variant win over those of the part.
    let get = |name: &str| variant.and_then(|v| attr(v, name)).or_else(|| attr(el, name));
    let text = |name: &str| get(name).map(language_text).unwrap_or_default();
    let number = text("P_ARTICLE_PARTNR");
    if number.is_empty() {
        return None;
    }
    let mut part = Part { number, ..Default::default() };
    part.type_number = text("P_ARTICLE_TYPENR");
    part.order_number = text("P_ARTICLE_ORDERNR");
    part.manufacturer = text("P_ARTICLE_MANUFACTURER");
    part.supplier = text("P_ARTICLE_SUPPLIER");
    part.description = text("P_ARTICLE_DESCR1");
    let more: Vec<String> = ["P_ARTICLE_DESCR2", "P_ARTICLE_DESCR3", "P_ARTICLE_NOTE"].iter().map(|n| text(n)).filter(|t| !t.is_empty()).collect();
    part.note = more.join("\n");
    part.identifier = text("P_ARTICLE_ERPNR");
    part.price = num(get("P_ARTICLE_SALESPRICE_1").or_else(|| get("P_ARTICLE_PURCHASEPRICE_1")));
    part.price_unit = num(get("P_ARTICLE_PRICEUNIT")).max(0.0) as u32;
    part.pack_quantity = num(get("P_ARTICLE_PACKAGINGQUANTITY")).max(0.0) as u32;
    part.quantity_unit = text("P_ARTICLE_QUANTITYUNIT");
    part.width = num(get("P_ARTICLE_WIDTH"));
    part.height = num(get("P_ARTICLE_HEIGHT"));
    part.depth = num(get("P_ARTICLE_DEPTH"));
    part.weight = num(get("P_ARTICLE_WEIGHT"));
    part.voltage = text("P_ARTICLE_VOLTAGE");
    part.current = text("P_ARTICLE_ELECTRICALCURRENT");
    part.power = text("P_ARTICLE_ELECTRICALPOWER");
    part.certificates = text("P_ARTICLE_CERTIFICATE");
    part.mounting = text("P_ARTICLE_MOUNTINGSITE");
    let mut tech: Vec<String> = Vec::new();
    for (key, name) in [("P_ARTICLE_CHARACTERISTICS", ""), ("P_ARTICLE_TECHNICALCHARACTERISTIC", ""), ("P_ARTICLE_POWERLOSS", "Power loss"), ("P_ARTICLE_COLOR", "Colour")] {
        let v = text(key);
        if !v.is_empty() {
            tech.push(if name.is_empty() { v } else { format!("{name}: {v}") });
        }
    }
    part.tech_data = tech.join("\n");
    let c = Classification {
        top_group: num(get("P_ARTICLE_PRODUCTTOPGROUP")) as u16,
        group: num(get("P_ARTICLE_PRODUCTGROUP")) as u16,
        subgroup: num(get("P_ARTICLE_PRODUCTSUBGROUP")) as u16,
    };
    productgroups::apply(&mut part, c);
    Some(part)
}

fn collect<'a>(el: &'a Element, out: &mut Vec<&'a Element>) {
    for c in &el.children {
        if c.name.eq_ignore_ascii_case("part") {
            out.push(c);
        } else {
            collect(c, out);
        }
    }
}

/// The parts of an EPLAN parts XML file; a part with variants gives one part per
/// variant with its own part number.
pub fn from_eplan_xml(text: &str) -> Result<Vec<Part>, String> {
    let root = xml::parse(text.trim_start_matches('\u{feff}'))?;
    let mut elements = Vec::new();
    if root.name.eq_ignore_ascii_case("part") {
        elements.push(&root);
    }
    collect(&root, &mut elements);
    let mut parts = Vec::new();
    for el in elements {
        let variants: Vec<&Element> = el.all("variant").collect();
        if variants.is_empty() {
            parts.extend(part_of(el, None));
        } else {
            parts.extend(variants.into_iter().filter_map(|v| part_of(el, Some(v))));
        }
    }
    if parts.is_empty() {
        return Err("the file has no EPLAN parts (P_ARTICLE_PARTNR)".into());
    }
    Ok(parts)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\n', "&#10;")
}

/// The parts as EPLAN parts XML (texts in `en_US`), for the import into EPLAN.
pub fn to_eplan_xml(parts: &[Part]) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<partsmanagement>\n");
    for p in parts {
        let c = productgroups::classify(p);
        let lang = |t: &str| if t.is_empty() { String::new() } else { format!("en_US@{t};") };
        let n = |v: f32| if v == 0.0 { String::new() } else { format!("{v}") };
        let attrs = [
            ("P_ARTICLE_PARTNR", p.number.clone()),
            ("P_ARTICLE_TYPENR", p.type_number.clone()),
            ("P_ARTICLE_ORDERNR", p.order_number.clone()),
            ("P_ARTICLE_MANUFACTURER", p.manufacturer.clone()),
            ("P_ARTICLE_SUPPLIER", p.supplier.clone()),
            ("P_ARTICLE_DESCR1", lang(&p.description)),
            ("P_ARTICLE_NOTE", lang(&p.note)),
            ("P_ARTICLE_ERPNR", p.identifier.clone()),
            ("P_ARTICLE_SALESPRICE_1", n(p.price)),
            ("P_ARTICLE_PRICEUNIT", if p.price_unit > 0 { p.price_unit.to_string() } else { String::new() }),
            ("P_ARTICLE_PACKAGINGQUANTITY", if p.pack_quantity > 0 { p.pack_quantity.to_string() } else { String::new() }),
            ("P_ARTICLE_QUANTITYUNIT", p.quantity_unit.clone()),
            ("P_ARTICLE_WIDTH", n(p.width)),
            ("P_ARTICLE_HEIGHT", n(p.height)),
            ("P_ARTICLE_DEPTH", n(p.depth)),
            ("P_ARTICLE_WEIGHT", n(p.weight)),
            ("P_ARTICLE_VOLTAGE", p.voltage.clone()),
            ("P_ARTICLE_ELECTRICALCURRENT", p.current.clone()),
            ("P_ARTICLE_ELECTRICALPOWER", p.power.clone()),
            ("P_ARTICLE_CERTIFICATE", p.certificates.clone()),
            ("P_ARTICLE_CHARACTERISTICS", lang(&p.tech_data.replace('\n', "; "))),
            ("P_ARTICLE_PRODUCTTOPGROUP", c.top_group.to_string()),
            ("P_ARTICLE_PRODUCTGROUP", c.group.to_string()),
            ("P_ARTICLE_PRODUCTSUBGROUP", c.subgroup.to_string()),
        ];
        out.push_str("  <part");
        for (k, v) in attrs.iter().filter(|(_, v)| !v.is_empty()) {
            let _ = write!(out, " {k}=\"{}\"", esc(v));
        }
        out.push_str(" />\n");
    }
    out.push_str("</partsmanagement>\n");
    out
}

#[cfg(test)]
mod tests;
