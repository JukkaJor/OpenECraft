//! Property database tools: property definitions, values of all objects, function
//! definitions, placement schemes and property library files.

use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::functions::{self, FnCategory, FunctionDef};
use openecraft_core::model::PlacedProp;
use openecraft_core::parts::FnKind;
use openecraft_core::properties::{self, DataType, Group, Library, Object, ObjectClass, Origin, PropDef};
use openecraft_core::props::{self, Scheme};
use openecraft_i18n::{tr, tr_const, tr_fmt};
use serde_json::{json, Map, Value};

const OBJECT: super::Param = param("object", Kind::Str, tr_const!("project, page:<name>, device:<tag>, function:<symbol id>, cable:<tag> or part:<number>"));

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "list_property_definitions",
        help: tr_const!("Lists the property definitions (built-in and user defined) with object class, group, type and unit"),
        params: &[
            optional("class", Kind::Str, tr_const!("project, page, device, function, terminal, plc, cable or part")),
            optional("filter", Kind::Str, tr_const!("Part of the key or name")),
        ],
        needs_project: true,
        run: list_definitions,
    },
    Tool {
        name: "define_property",
        help: tr_const!("Defines a user property or changes it"),
        params: &[
            param("key", Kind::Str, tr_const!("Key, e.g. user.customer_code")),
            param("name", Kind::Str, tr_const!("Name")),
            param("class", Kind::Str, tr_const!("project, page, device, function, terminal, plc or cable")),
            optional("type", Kind::Str, tr_const!("text, multiline, integer, number, bool, choice, date or link (default text)")),
            optional("unit", Kind::Str, tr_const!("Unit, e.g. kW")),
            optional("choices", Kind::Str, tr_const!("Values of a selection list separated by commas")),
            optional("indices", Kind::Int, tr_const!("Number of indexed values (0 = a single value)")),
            optional("default", Kind::Str, tr_const!("Value while none is set")),
            optional("description", Kind::Str, tr_const!("Description")),
        ],
        needs_project: true,
        run: define_property,
    },
    Tool {
        name: "remove_property_definition",
        help: tr_const!("Removes a user property with all its values and placements"),
        params: &[param("key", Kind::Str, tr_const!("Key"))],
        needs_project: true,
        run: remove_definition,
    },
    Tool {
        name: "get_properties",
        help: tr_const!("Lists the properties of an object with their values and where they come from"),
        params: &[
            OBJECT,
            optional("all", Kind::Bool, tr_const!("Also properties without a value")),
            optional("filter", Kind::Str, tr_const!("Part of the key or name")),
        ],
        needs_project: true,
        run: get_properties,
    },
    Tool {
        name: "set_properties",
        help: tr_const!("Sets property values of an object; an empty value removes it"),
        params: &[OBJECT, param("values", Kind::Object, tr_const!("Property keys and values, e.g. {\"device.rated_power\": \"5.5\"}"))],
        needs_project: true,
        run: set_properties,
    },
    Tool {
        name: "list_function_definitions",
        help: tr_const!("Lists the function definitions with category, function template kind, letter, default symbol and usage"),
        params: &[optional("filter", Kind::Str, tr_const!("Part of the key or name"))],
        needs_project: true,
        run: list_functions,
    },
    Tool {
        name: "define_function",
        help: tr_const!("Defines a reusable project function definition or changes it"),
        params: &[
            param("key", Kind::Str, tr_const!("Key, e.g. user.coupling_relay")),
            optional("from", Kind::Str, tr_const!("Definition to start from")),
            optional("name", Kind::Str, tr_const!("Name")),
            optional("letter", Kind::Str, tr_const!("Identifying letter")),
            optional("symbol", Kind::Str, tr_const!("Default symbol key")),
            optional("pins", Kind::Str, tr_const!("Connection point designations separated by blanks")),
            optional("scheme", Kind::Str, tr_const!("Placement scheme")),
            optional("defaults", Kind::Object, tr_const!("Default property values")),
            optional("description", Kind::Str, tr_const!("Description")),
        ],
        needs_project: true,
        run: define_function,
    },
    Tool {
        name: "remove_function_definition",
        help: tr_const!("Removes a project function definition; its symbols go back to their built-in one"),
        params: &[param("key", Kind::Str, tr_const!("Key"))],
        needs_project: true,
        run: remove_function,
    },
    Tool {
        name: "assign_function_definition",
        help: tr_const!("Assigns a function definition to a placed symbol (designations and scheme are taken over)"),
        params: &[param("id", Kind::Int, tr_const!("Symbol id")), param("function", Kind::Str, tr_const!("Function definition key"))],
        needs_project: true,
        run: assign_function,
    },
    Tool {
        name: "list_property_schemes",
        help: tr_const!("Lists the placement schemes with their properties and the number of symbols using them"),
        params: &[],
        needs_project: true,
        run: list_schemes,
    },
    Tool {
        name: "save_property_scheme",
        help: tr_const!("Saves the properties placed at a symbol as a placement scheme"),
        params: &[param("name", Kind::Str, tr_const!("Name of the scheme")), param("id", Kind::Int, tr_const!("Symbol id"))],
        needs_project: true,
        run: save_scheme,
    },
    Tool {
        name: "apply_property_scheme",
        help: tr_const!("Places the properties of a scheme at symbols; without ids it is placed again at all symbols using it"),
        params: &[param("name", Kind::Str, tr_const!("Name of the scheme")), optional("ids", Kind::Ids, tr_const!("Symbol ids"))],
        needs_project: true,
        run: apply_scheme,
    },
    Tool {
        name: "place_property",
        help: tr_const!("Shows a property as a text beside a symbol"),
        params: &[
            param("id", Kind::Int, tr_const!("Symbol id")),
            param("key", Kind::Str, tr_const!("Property key")),
            optional("dx", Kind::Num, tr_const!("Offset from the symbol position in mm (default: below the symbol)")),
            optional("dy", Kind::Num, tr_const!("Offset from the symbol position in mm (default: below the symbol)")),
            optional("size", Kind::Num, tr_const!("Text height in mm")),
            optional("name", Kind::Bool, tr_const!("Show as \"Name: value\"")),
        ],
        needs_project: true,
        run: place_property,
    },
    Tool {
        name: "export_property_library",
        help: tr_const!("Writes the project's property definitions, function definitions and schemes as a library file"),
        params: &[optional("path", Kind::Str, tr_const!("Output file; without it the content is returned"))],
        needs_project: true,
        run: export_library,
    },
    Tool {
        name: "import_property_library",
        help: tr_const!("Adds the definitions and schemes of a property library file to the project"),
        params: &[param("path", Kind::Str, tr_const!("Library file"))],
        needs_project: true,
        run: import_library,
    },
];

fn class_of(text: &str) -> Result<ObjectClass, String> {
    ObjectClass::from_key(text).ok_or_else(|| tr_fmt!("Unknown object class: {}", text))
}

fn def_json(d: &PropDef) -> Value {
    json!({
        "key": d.key,
        "name": d.title(),
        "class": d.class.key(),
        "group": d.group.title(),
        "type": d.ty.key(),
        "unit": d.unit,
        "choices": d.choices,
        "indices": d.indices,
        "read_only": d.read_only,
        "builtin": d.builtin,
    })
}

fn list_definitions(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let class = a.str("class").map(class_of).transpose()?;
    let f = a.str("filter").unwrap_or_default().to_lowercase();
    let list = properties::all_defs(prj)
        .filter(|d| class.is_none_or(|c| d.class == c))
        .filter(|d| f.is_empty() || d.key.contains(&f) || d.title().to_lowercase().contains(&f))
        .map(def_json);
    Ok(Value::Array(list.collect()))
}

fn define_property(s: &mut Session, a: &Args) -> Result<Value, String> {
    let ty = match a.str("type") {
        Some(t) => DataType::from_key(t).ok_or_else(|| tr_fmt!("Unknown data type: {}", t))?,
        None => DataType::Text,
    };
    let d = PropDef {
        key: a.req("key").trim().into(),
        name: a.req("name").trim().into(),
        class: class_of(a.req("class"))?,
        group: Group::User,
        ty,
        unit: a.str("unit").unwrap_or_default().into(),
        choices: a.str("choices").unwrap_or_default().split(',').map(|c| c.trim().to_string()).filter(|c| !c.is_empty()).collect(),
        indices: a.int("indices").unwrap_or(0).clamp(0, 500) as u16,
        default: a.str("default").unwrap_or_default().into(),
        description: a.str("description").unwrap_or_default().into(),
        ..Default::default()
    };
    let prj = s.project_mut()?;
    properties::define(prj, d)?;
    Ok(def_json(properties::def(prj, a.req("key").trim()).unwrap()))
}

fn remove_definition(s: &mut Session, a: &Args) -> Result<Value, String> {
    let n = properties::undefine(s.project_mut()?, a.req("key"))?;
    Ok(json!({ "key": a.req("key"), "values_removed": n }))
}

fn origin_key(o: Origin) -> &'static str {
    match o {
        Origin::None => "none",
        Origin::Stored => "set",
        Origin::Model => "model",
        Origin::FunctionDefinition => "function_definition",
        Origin::Default => "default",
    }
}

fn get_properties(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let obj = Object::parse(prj, a.req("object"))?;
    let f = a.str("filter").unwrap_or_default().to_lowercase();
    let list = properties::keys_of(prj, &obj, false, !a.flag("all"))
        .into_iter()
        .filter(|(k, d)| f.is_empty() || k.contains(&f) || d.title_of(k).to_lowercase().contains(&f))
        .map(|(k, d)| {
            json!({
                "key": k,
                "name": d.title_of(&k),
                "value": properties::get(prj, &obj, &k),
                "display": properties::display(prj, &obj, &k),
                "origin": origin_key(properties::origin(prj, &obj, &k)),
                "editable": d.editable() && d.class != ObjectClass::Part,
            })
        });
    let function = properties::function_def(prj, &obj).map(|f| f.key.clone());
    Ok(json!({ "object": obj.title(prj), "function_definition": function, "properties": list.collect::<Vec<_>>() }))
}

fn set_properties(s: &mut Session, a: &Args) -> Result<Value, String> {
    let values: Map<String, Value> = a.value("values").and_then(Value::as_object).cloned().unwrap_or_default();
    let prj = s.project_mut()?;
    let obj = Object::parse(prj, a.req("object"))?;
    let mut set = Map::new();
    for (k, v) in &values {
        let text = match v {
            Value::String(t) => t.clone(),
            Value::Null => String::new(),
            other => other.to_string(),
        };
        properties::set(prj, &obj, k, &text)?;
        set.insert(k.clone(), Value::String(properties::get(prj, &obj, k)));
    }
    Ok(json!({ "object": obj.title(prj), "values": set }))
}

fn function_json(d: &FunctionDef, used: usize) -> Value {
    json!({
        "key": d.key,
        "name": d.title(),
        "category": d.category.title(),
        "kind": format!("{:?}", d.kind),
        "letter": d.letter,
        "symbol": d.symbol,
        "pins": d.pins,
        "scheme": d.scheme,
        "defaults": d.defaults,
        "builtin": d.builtin,
        "used": used,
    })
}

fn list_functions(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let usage = functions::usage(prj);
    let f = a.str("filter").unwrap_or_default().to_lowercase();
    let list = functions::all(prj)
        .filter(|d| f.is_empty() || d.key.contains(&f) || d.title().to_lowercase().contains(&f))
        .map(|d| function_json(d, usage.get(&d.key).copied().unwrap_or(0)));
    Ok(Value::Array(list.collect()))
}

fn define_function(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let key = a.req("key").trim().to_string();
    let base = match a.str("from") {
        Some(from) => functions::get(prj, from).cloned().ok_or_else(|| tr_fmt!("Unknown function definition: {}", from))?,
        None => functions::get(prj, &key).cloned().unwrap_or_else(|| FunctionDef { category: FnCategory::General, kind: FnKind::Generic, ..Default::default() }),
    };
    let mut d = FunctionDef { key: key.clone(), name: base.title(), builtin: false, ..base };
    if let Some(v) = a.str("name") {
        d.name = v.into();
    }
    if let Some(v) = a.str("letter") {
        d.letter = v.into();
    }
    if let Some(v) = a.str("symbol") {
        d.symbol = v.into();
    }
    if let Some(v) = a.str("pins") {
        d.pins = v.into();
    }
    if let Some(v) = a.str("scheme") {
        if !v.is_empty() && props::scheme(prj, v).is_none() {
            return Err(tr_fmt!("Unknown scheme: {}", v));
        }
        d.scheme = v.into();
    }
    if let Some(v) = a.str("description") {
        d.description = v.into();
    }
    if let Some(defaults) = a.value("defaults").and_then(Value::as_object) {
        for (k, v) in defaults {
            let text = v.as_str().map(String::from).unwrap_or_else(|| v.to_string());
            let pd = properties::valid_key(prj, k).ok_or_else(|| tr_fmt!("Unknown property: {}", k))?;
            let text = pd.check(&text)?;
            if text.is_empty() {
                d.defaults.remove(k);
            } else {
                d.defaults.insert(k.clone(), text);
            }
        }
    }
    functions::define(prj, d)?;
    Ok(function_json(functions::get(prj, &key).unwrap(), 0))
}

fn remove_function(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let key = a.req("key");
    if !prj.property_db.functions.iter().any(|d| d.key == key) {
        return Err(tr_fmt!("No project function definition {}", key));
    }
    let n = functions::remove(prj, key);
    Ok(json!({ "key": key, "symbols_reset": n }))
}

fn symbol_id(a: &Args) -> Result<u64, String> {
    a.int("id").and_then(|i| u64::try_from(i).ok()).ok_or_else(|| tr_fmt!("Missing parameter: {}", "id"))
}

fn assign_function(s: &mut Session, a: &Args) -> Result<Value, String> {
    let id = symbol_id(a)?;
    let prj = s.project_mut()?;
    functions::assign(prj, id, a.req("function"))?;
    let (_, sym) = properties::find_symbol(prj, id).unwrap();
    let f = functions::of_symbol(prj, sym).map(|d| d.key.clone());
    Ok(json!({ "id": id, "function": f, "pins": sym.pins, "scheme": sym.scheme }))
}

fn scheme_json(sc: &Scheme, used: usize) -> Value {
    let entries: Vec<Value> = sc.entries.iter().map(|e| json!({ "key": e.key, "dx": e.offset.x, "dy": e.offset.y, "size": e.size, "name": e.name })).collect();
    json!({ "name": sc.name, "title": sc.title(), "builtin": sc.builtin, "entries": entries, "used": used })
}

fn list_schemes(s: &mut Session, _: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let used = |name: &str| prj.pages.iter().flat_map(|p| p.symbols.iter()).filter(|s| s.scheme == name).count();
    Ok(Value::Array(props::all_schemes(prj).map(|sc| scheme_json(sc, used(&sc.name))).collect()))
}

fn save_scheme(s: &mut Session, a: &Args) -> Result<Value, String> {
    let symbols = s.project()?.symbol_table();
    let id = symbol_id(a)?;
    let prj = s.project_mut()?;
    let (_, sym) = properties::find_symbol(prj, id).ok_or_else(|| tr_fmt!("No symbol with id {}", id))?;
    let scheme = props::capture(&symbols, sym, a.req("name"));
    let name = scheme.name.clone();
    props::define_scheme(prj, scheme)?;
    if let Some(sym) = prj.pages.iter_mut().flat_map(|p| p.symbols.iter_mut()).find(|x| x.id == id) {
        sym.scheme = name.clone();
    }
    Ok(scheme_json(props::scheme(prj, &name).unwrap(), 1))
}

fn apply_scheme(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let name = a.req("name");
    let ids = a.ids("ids");
    let n = if ids.is_empty() {
        if props::scheme(prj, name).is_none() {
            return Err(tr_fmt!("Unknown scheme: {}", name));
        }
        props::reapply(prj, name)
    } else {
        props::apply_to(prj, name, &ids)?
    };
    Ok(json!({ "name": name, "symbols": n }))
}

fn place_property(s: &mut Session, a: &Args) -> Result<Value, String> {
    let symbols = s.project()?.symbol_table();
    let id = symbol_id(a)?;
    let prj = s.project_mut()?;
    let key = a.req("key");
    if properties::valid_key(prj, key).is_none() {
        return Err(tr_fmt!("Unknown property: {}", key));
    }
    let (pi, si) = prj
        .pages
        .iter()
        .enumerate()
        .find_map(|(pi, pg)| pg.symbols.iter().position(|x| x.id == id).map(|si| (pi, si)))
        .ok_or_else(|| tr_fmt!("No symbol with id {}", id))?;
    if !prj.pages[pi].is_editable() {
        return Err(tr("The page is released").into());
    }
    let sym = &mut prj.pages[pi].symbols[si];
    props::add(&symbols, sym, key);
    let pp: &mut PlacedProp = sym.props.last_mut().unwrap();
    if let Some(dx) = a.num("dx") {
        pp.offset.x = dx;
    }
    if let Some(dy) = a.num("dy") {
        pp.offset.y = dy;
    }
    if let Some(size) = a.num("size") {
        pp.size = size.clamp(1.0, 10.0);
    }
    pp.name = a.flag("name");
    let sym = &prj.pages[pi].symbols[si];
    let text = props::text(prj, pi, sym, sym.props.last().unwrap());
    Ok(json!({ "id": id, "key": key, "text": text, "placed": sym.props.len() }))
}

fn export_library(s: &mut Session, a: &Args) -> Result<Value, String> {
    let content = s.project()?.property_db.to_json();
    match a.str("path") {
        Some(path) => {
            std::fs::write(path, &content).map_err(|e| tr_fmt!("Cannot write {}: {}", path, e))?;
            Ok(json!({ "path": path, "bytes": content.len() }))
        }
        None => serde_json::from_str(&content).map_err(|e| e.to_string()),
    }
}

fn import_library(s: &mut Session, a: &Args) -> Result<Value, String> {
    let path = a.req("path");
    let text = std::fs::read_to_string(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, e))?;
    let lib = Library::from_json(&text)?;
    let n = s.project_mut()?.property_db.merge(lib);
    Ok(json!({ "path": path, "imported": n }))
}

#[cfg(test)]
mod tests;
