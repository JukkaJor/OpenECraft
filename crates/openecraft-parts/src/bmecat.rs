//! BMEcat import (1.2 and 2005): the product catalogs manufacturers and wholesalers
//! publish, with ETIM / eCl@ss classification and features. Each product becomes a
//! part: identification, description, price, data sheet, the class and all features
//! as technical data; dimensions, weight, rated current / voltage and power loss go
//! into their fields where the feature names (or the ETIM codes) are known.

use crate::xml::{self, Element};
use crate::{Part, TopGroup};

/// UN/ECE unit codes used in catalogs and their symbols.
const UNITS: [(&str, &str); 14] = [
    ("MMT", "mm"),
    ("CMT", "cm"),
    ("MTR", "m"),
    ("GRM", "g"),
    ("KGM", "kg"),
    ("AMP", "A"),
    ("MAM", "mA"),
    ("VLT", "V"),
    ("WTT", "W"),
    ("KWT", "kW"),
    ("CEL", "°C"),
    ("HTZ", "Hz"),
    ("MMK", "mm²"),
    ("C62", ""),
];

fn unit_symbol(code: &str) -> &str {
    UNITS.iter().find(|u| u.0.eq_ignore_ascii_case(code)).map(|u| u.1).unwrap_or(code)
}

/// What a feature means for the part's own fields: by name (English / German) or by
/// its ETIM feature code.
#[derive(Clone, Copy, PartialEq)]
enum Meaning {
    Width,
    Height,
    Depth,
    Weight,
    Current,
    Voltage,
    PowerLoss,
    Other,
}

fn meaning(name: &str) -> Meaning {
    let n = name.trim().to_lowercase();
    match n.as_str() {
        "width" | "breite" | "ef000008" => Meaning::Width,
        "height" | "höhe" | "hoehe" | "ef000040" => Meaning::Height,
        "depth" | "tiefe" | "ef000049" => Meaning::Depth,
        "weight" | "gewicht" | "net weight" | "nettogewicht" => Meaning::Weight,
        _ if n.contains("power loss") || n.contains("verlustleistung") => Meaning::PowerLoss,
        _ if n.contains("rated current") || n.contains("rated operational current") || n.contains("bemessungsstrom") || n.contains("bemessungsbetriebsstrom") => Meaning::Current,
        _ if n.contains("rated voltage") || n.contains("control supply voltage") || n.contains("bemessungsspannung") || n.contains("steuerspeisespannung") => Meaning::Voltage,
        _ => Meaning::Other,
    }
}

fn number(v: &str) -> Option<f32> {
    v.trim().replace(',', ".").parse().ok()
}

/// A length in mm from a value and a unit symbol.
fn to_mm(v: f32, unit: &str) -> f32 {
    match unit {
        "cm" => v * 10.0,
        "m" => v * 1000.0,
        _ => v,
    }
}

/// Short manufacturer prefix for part numbers: `Phoenix Contact GmbH` → `PHOENIX`.
fn prefix(manufacturer: &str) -> String {
    manufacturer.split_whitespace().next().unwrap_or("").chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_uppercase()
}

/// The first child of `el` with one of the names (BMEcat 2005 / 1.2 spelling).
fn either<'a>(el: &'a Element, names: &[&str]) -> Option<&'a Element> {
    names.iter().find_map(|n| el.child(n))
}

fn text_either<'a>(el: &'a Element, names: &[&str]) -> &'a str {
    names.iter().map(|n| el.text_of(n)).find(|t| !t.is_empty()).unwrap_or("")
}

fn product(p: &Element, supplier: &str, currency: &str) -> Option<Part> {
    let supplier_id = text_either(p, &["SUPPLIER_PID", "SUPPLIER_AID"]);
    let details = either(p, &["PRODUCT_DETAILS", "ARTICLE_DETAILS"]);
    let d = |names: &[&str]| details.map(|e| text_either(e, names)).unwrap_or("").to_string();
    let manufacturer = d(&["MANUFACTURER_NAME"]);
    let manufacturer_id = d(&["MANUFACTURER_PID", "MANUFACTURER_AID"]);
    let type_number = if manufacturer_id.is_empty() { d(&["MANUFACTURER_TYPE_DESCR"]) } else { manufacturer_id.clone() };
    let id = if type_number.is_empty() { supplier_id.to_string() } else { type_number.clone() };
    if id.is_empty() {
        return None;
    }
    let pre = prefix(if manufacturer.is_empty() { supplier } else { &manufacturer });
    let mut part = Part {
        number: if pre.is_empty() { id } else { format!("{pre}.{id}") },
        type_number,
        order_number: supplier_id.to_string(),
        manufacturer,
        supplier: supplier.to_string(),
        description: d(&["DESCRIPTION_SHORT"]),
        note: d(&["DESCRIPTION_LONG"]),
        top_group: TopGroup::Electrical,
        currency: currency.to_string(),
        ..Default::default()
    };
    let mut tech: Vec<String> = Vec::new();
    let ean = details.map(|e| {
        let mut v = Vec::new();
        e.descendants("INTERNATIONAL_PID", &mut v);
        v.first().map(|x| x.text.trim().to_string()).unwrap_or_else(|| e.text_of("EAN").to_string())
    });
    if let Some(ean) = ean.filter(|e| !e.is_empty()) {
        tech.push(format!("GTIN: {ean}"));
    }
    for f in p.all("PRODUCT_FEATURES").chain(p.all("ARTICLE_FEATURES")) {
        let system = f.text_of("REFERENCE_FEATURE_SYSTEM_NAME");
        let class = f.text_of("REFERENCE_FEATURE_GROUP_ID");
        let class_name = f.text_of("REFERENCE_FEATURE_GROUP_NAME");
        if !class.is_empty() {
            let kind = if system.to_uppercase().starts_with("ECLASS") || system.to_lowercase().starts_with("ecl@ss") { "eCl@ss class" } else { "ETIM class" };
            let named = if class_name.is_empty() { String::new() } else { format!(" {class_name}") };
            tech.push(format!("{kind}: {class}{named} ({system})"));
            if part.group.is_empty() && !class_name.is_empty() {
                part.group = class_name.to_string();
            }
        }
        for feat in f.all("FEATURE") {
            let code = feat.text_of("FNAME");
            let descr = feat.text_of("FDESCR");
            let values: Vec<&str> = feat.all("FVALUE").map(|v| v.text.trim()).filter(|v| !v.is_empty() && *v != "-").collect();
            if code.is_empty() || values.is_empty() {
                continue;
            }
            let unit = unit_symbol(feat.text_of("FUNIT")).to_string();
            let value = values.join(", ");
            let shown = if unit.is_empty() { value.clone() } else { format!("{value} {unit}") };
            let what = if meaning(code) != Meaning::Other { meaning(code) } else { meaning(descr) };
            let v = number(&value);
            match (what, v) {
                (Meaning::Width, Some(v)) => part.width = to_mm(v, &unit),
                (Meaning::Height, Some(v)) => part.height = to_mm(v, &unit),
                (Meaning::Depth, Some(v)) => part.depth = to_mm(v, &unit),
                (Meaning::Weight, Some(v)) => part.weight = if unit == "g" { v / 1000.0 } else { v },
                (Meaning::Current, _) if part.current.is_empty() => part.current = shown.clone(),
                (Meaning::Voltage, _) if part.voltage.is_empty() => part.voltage = shown.clone(),
                (Meaning::PowerLoss, Some(_)) => tech.push(format!("Power loss: {shown}")),
                _ => {}
            }
            let name = if descr.is_empty() { code.to_string() } else { format!("{descr} ({code})") };
            if !name.eq_ignore_ascii_case("Power loss") {
                tech.push(format!("{name}: {shown}"));
            }
        }
    }
    part.tech_data = tech.join("\n");
    if let Some(order) = either(p, &["PRODUCT_ORDER_DETAILS", "ARTICLE_ORDER_DETAILS"]) {
        part.quantity_unit = order.text_of("ORDER_UNIT").to_string();
        part.pack_quantity = order.text_of("NO_CU_PER_OU").parse::<f32>().map(|v| v.max(1.0) as u32).unwrap_or(1);
        part.price_unit = order.text_of("PRICE_QUANTITY").parse::<f32>().map(|v| v.max(1.0) as u32).unwrap_or(1);
    }
    if let Some(prices) = either(p, &["PRODUCT_PRICE_DETAILS", "ARTICLE_PRICE_DETAILS"]) {
        let all: Vec<&Element> = prices.all("PRODUCT_PRICE").chain(prices.all("ARTICLE_PRICE")).collect();
        // Prefer the net list price.
        let price = all.iter().find(|x| x.attribute("price_type").is_some_and(|t| t.contains("net_list"))).or(all.first());
        if let Some(price) = price {
            part.price = number(price.text_of("PRICE_AMOUNT")).unwrap_or(0.0);
            let c = price.text_of("PRICE_CURRENCY");
            if !c.is_empty() {
                part.currency = c.to_string();
            }
        }
    }
    if let Some(mime) = p.child("MIME_INFO") {
        let docs: Vec<&Element> = mime.all("MIME").collect();
        let sheet = docs
            .iter()
            .find(|m| m.text_of("MIME_PURPOSE") == "data_sheet")
            .or_else(|| docs.iter().find(|m| m.text_of("MIME_TYPE") == "application/pdf"));
        if let Some(m) = sheet {
            part.datasheet = m.text_of("MIME_SOURCE").to_string();
        }
    }
    Some(part)
}

/// Supplier ids of the products a product refers to as its accessories
/// (`accessories`) and as the products it is an accessory of (`base_product`).
type References = (Vec<String>, Vec<String>);

fn references(p: &Element) -> References {
    let (mut accessories, mut bases) = (Vec::new(), Vec::new());
    for r in p.all("PRODUCT_REFERENCE").chain(p.all("ARTICLE_REFERENCE")) {
        let to = text_either(r, &["PROD_ID_TO", "ART_ID_TO"]).trim().to_string();
        match r.attribute("type").unwrap_or_default() {
            _ if to.is_empty() => {}
            "accessories" => accessories.push(to),
            "base_product" => bases.push(to),
            _ => {}
        }
    }
    (accessories, bases)
}

/// Whether a text looks like a BMEcat document.
pub fn is_bmecat(text: &str) -> bool {
    let head: String = text.trim_start_matches('\u{feff}').trim_start().chars().take(2000).collect();
    head.starts_with('<') && head.contains("BMECAT")
}

/// Reads the products (BMEcat 2005) or articles (1.2) of a catalog as parts.
pub fn from_bmecat(text: &str) -> Result<Vec<Part>, String> {
    let root = xml::parse(text)?;
    if root.name != "BMECAT" {
        return Err("not a BMEcat document".into());
    }
    let header = root.child("HEADER");
    let supplier = header.and_then(|h| h.child("SUPPLIER")).map(|s| text_either(s, &["SUPPLIER_NAME"])).unwrap_or("").to_string();
    let currency = header.and_then(|h| h.child("CATALOG")).map(|c| c.text_of("CURRENCY")).unwrap_or("").to_string();
    let mut products = Vec::new();
    root.descendants("PRODUCT", &mut products);
    root.descendants("ARTICLE", &mut products);
    let found: Vec<(Part, References)> = products.into_iter().filter_map(|p| Some((product(p, &supplier, &currency)?, references(p)))).collect();
    if found.is_empty() {
        return Err("the catalog has no products".into());
    }
    // References name supplier ids: the part number of a product of the catalog, else
    // the number such a product would get.
    let number_of = |id: &str| {
        found.iter().find(|(p, _)| p.order_number == id).map(|(p, _)| p.number.clone()).unwrap_or_else(|| {
            let pre = prefix(&supplier);
            if pre.is_empty() { id.to_string() } else { format!("{pre}.{id}") }
        })
    };
    let resolved: Vec<References> = found.iter().map(|(_, (acc, bases))| (acc.iter().map(|id| number_of(id)).collect(), bases.iter().map(|id| number_of(id)).collect())).collect();
    Ok(found
        .into_iter()
        .zip(resolved)
        .map(|((mut part, _), (accessories, fits))| {
            part.accessories = accessories;
            part.accessory = !fits.is_empty();
            part.fits = fits;
            part
        })
        .collect())
}

#[cfg(test)]
mod tests;
