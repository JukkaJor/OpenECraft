//! Project options: model content (pages, symbols, wires, texts, graphics …) that
//! belongs to an option is only part of the project while the option is active. An
//! inactive option keeps its content parked, so drawing, evaluation, reports and
//! exports leave it out; activating the option puts the content back where it was.
//! An option may follow a condition on the configurator options.

use openecraft_i18n::{tr, tr_fmt};
use crate::model::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelOption {
    pub name: String,
    pub description: String,
    /// Configurator condition (`OPTION=value & …`); empty: switched by hand.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub condition: String,
    pub active: bool,
    /// Ids of the pages and page items that belong to the option.
    pub members: BTreeSet<u64>,
    /// Content of the inactive option, in the order it was taken out.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parked: Vec<Parked>,
}

impl Default for ModelOption {
    fn default() -> Self {
        ModelOption { name: String::new(), description: String::new(), condition: String::new(), active: true, members: BTreeSet::new(), parked: Vec::new() }
    }
}

/// Content of an inactive option.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Page(Box<Page>),
    Symbol(Box<SymbolInst>),
    Wire(Box<Wire>),
    Text(TextItem),
    Shape(Shape),
    Dimension(Dimension),
    Image(ImageItem),
    ConnDef(ConnDef),
    CableDef(CableDef),
}

impl Item {
    pub fn id(&self) -> u64 {
        match self {
            Item::Page(x) => x.id,
            Item::Symbol(x) => x.id,
            Item::Wire(x) => x.id,
            Item::Text(x) => x.id,
            Item::Shape(x) => x.id,
            Item::Dimension(x) => x.id,
            Item::Image(x) => x.id,
            Item::ConnDef(x) => x.id,
            Item::CableDef(x) => x.id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Parked {
    /// Page the item was on; pages: the page before it (0 = the first page).
    pub page: u64,
    /// Position in its list.
    pub index: usize,
    pub item: Item,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ModelOptions {
    pub options: Vec<ModelOption>,
}

impl ModelOptions {
    pub fn is_empty(&self) -> bool {
        self.options.is_empty()
    }

    pub fn get(&self, name: &str) -> Option<&ModelOption> {
        self.options.iter().find(|o| o.name == name)
    }

    /// Option an object (page or page item) belongs to.
    pub fn option_of(&self, id: u64) -> Option<&ModelOption> {
        self.options.iter().find(|o| o.members.contains(&id))
    }

    /// Largest id of the parked content.
    pub fn max_id(&self) -> u64 {
        let mut max = 0;
        for p in self.options.iter().flat_map(|o| &o.parked) {
            max = max.max(p.item.id());
            if let Item::Page(pg) = &p.item {
                max = max.max(page_max_id(pg));
            }
        }
        max
    }

    /// Parked content on or of the given pages (option names).
    pub fn parked_on(&self, pages: &BTreeSet<u64>) -> Vec<&str> {
        let on = |p: &Parked| pages.contains(&p.page) && !matches!(p.item, Item::Page(_)) || pages.contains(&p.item.id());
        self.options.iter().filter(|o| o.parked.iter().any(on)).map(|o| o.name.as_str()).collect()
    }
}

fn page_max_id(pg: &Page) -> u64 {
    let mut ids = vec![pg.id];
    ids.extend(pg.symbols.iter().map(|x| x.id));
    ids.extend(pg.wires.iter().map(|x| x.id));
    ids.extend(pg.texts.iter().map(|x| x.id));
    ids.extend(pg.shapes.iter().map(|x| x.id));
    ids.extend(pg.dimensions.iter().map(|x| x.id));
    ids.extend(pg.images.iter().map(|x| x.id));
    ids.extend(pg.conn_defs.iter().map(|x| x.id));
    ids.extend(pg.cable_defs.iter().map(|x| x.id));
    ids.into_iter().max().unwrap_or(0)
}

/// Ids of a page's items (not the page itself).
pub fn item_ids(pg: &Page) -> Vec<u64> {
    let mut ids: Vec<u64> = pg.symbols.iter().map(|x| x.id).collect();
    ids.extend(pg.wires.iter().map(|x| x.id));
    ids.extend(pg.texts.iter().map(|x| x.id));
    ids.extend(pg.shapes.iter().map(|x| x.id));
    ids.extend(pg.dimensions.iter().map(|x| x.id));
    ids.extend(pg.images.iter().map(|x| x.id));
    ids.extend(pg.conn_defs.iter().map(|x| x.id));
    ids.extend(pg.cable_defs.iter().map(|x| x.id));
    ids
}

fn option_mut<'a>(prj: &'a mut Project, name: &str) -> Result<&'a mut ModelOption, String> {
    prj.model_options.options.iter_mut().find(|o| o.name == name).ok_or_else(|| tr_fmt!("No option {}", name))
}

fn check_condition(prj: &Project, condition: &str) -> Result<(), String> {
    if !condition.trim().is_empty() {
        prj.configuration.holds(condition)?;
    }
    Ok(())
}

/// Adds an option or changes its description and condition.
pub fn define(prj: &mut Project, name: &str, description: &str, condition: &str) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(tr("The option needs a name").into());
    }
    check_condition(prj, condition)?;
    match prj.model_options.options.iter_mut().find(|o| o.name == name) {
        Some(o) => {
            o.description = description.into();
            o.condition = condition.trim().into();
        }
        None => prj.model_options.options.push(ModelOption { name: name.into(), description: description.into(), condition: condition.trim().into(), ..Default::default() }),
    }
    Ok(())
}

/// Removes an option; its content is put back and stays in the project.
pub fn remove(prj: &mut Project, name: &str) -> Result<(), String> {
    set_active(prj, name, true)?;
    prj.model_options.options.retain(|o| o.name != name);
    Ok(())
}

/// Makes pages or page items content of an option (each belongs to one option at
/// most). Content of an inactive option is taken out at once. Returns the number of
/// objects assigned.
pub fn assign(prj: &mut Project, name: &str, ids: &[u64]) -> Result<usize, String> {
    let target = prj.model_options.get(name).ok_or_else(|| tr_fmt!("No option {}", name))?;
    let active = target.active;
    for &id in ids {
        if !in_pages(prj, id) {
            return Err(tr_fmt!("No page or page item with id {}", id));
        }
    }
    for o in &mut prj.model_options.options {
        for id in ids {
            o.members.remove(id);
        }
    }
    let ids: BTreeSet<u64> = ids.iter().copied().collect();
    option_mut(prj, name)?.members.extend(ids.iter().copied());
    if !active {
        if prj.pages.iter().all(|p| ids.contains(&p.id)) {
            option_mut(prj, name)?.members.retain(|id| !ids.contains(id));
            return Err(tr("An option cannot hide all pages").into());
        }
        let parked = take(prj, &ids);
        option_mut(prj, name)?.parked.extend(parked);
    }
    Ok(ids.len())
}

/// Content no longer belongs to an option; parked content is put back.
pub fn unassign(prj: &mut Project, ids: &[u64]) -> usize {
    let mut n = 0;
    let mut back = Vec::new();
    for o in &mut prj.model_options.options {
        for id in ids {
            if o.members.remove(id) {
                n += 1;
            }
        }
        let (out, keep): (Vec<Parked>, Vec<Parked>) = std::mem::take(&mut o.parked).into_iter().partition(|p| ids.contains(&p.item.id()));
        o.parked = keep;
        back.extend(out);
    }
    put_back(prj, back);
    n
}

/// Activates (puts back) or deactivates (takes out) the content of an option.
/// Returns the number of objects moved.
pub fn set_active(prj: &mut Project, name: &str, active: bool) -> Result<usize, String> {
    let o = prj.model_options.get(name).ok_or_else(|| tr_fmt!("No option {}", name))?;
    if o.active == active {
        return Ok(0);
    }
    if active {
        let o = option_mut(prj, name)?;
        o.active = true;
        let parked = std::mem::take(&mut o.parked);
        let n = parked.len();
        put_back(prj, parked);
        Ok(n)
    } else {
        let members = o.members.clone();
        if prj.pages.iter().all(|p| members.contains(&p.id)) {
            return Err(tr("An option cannot hide all pages").into());
        }
        let parked = take(prj, &members);
        let o = option_mut(prj, name)?;
        o.active = false;
        let n = parked.len();
        o.parked = parked;
        Ok(n)
    }
}

/// Sets the options with a condition by the configurator options. Returns the
/// options switched with their new state.
pub fn apply_conditions(prj: &mut Project) -> Result<Vec<(String, bool)>, String> {
    let wanted: Vec<(String, bool)> = prj
        .model_options
        .options
        .iter()
        .filter(|o| !o.condition.trim().is_empty())
        .map(|o| Ok((o.name.clone(), prj.configuration.holds(&o.condition)?)))
        .collect::<Result<_, String>>()?;
    let mut switched = Vec::new();
    for (name, active) in wanted {
        if prj.model_options.get(&name).is_some_and(|o| o.active != active) {
            set_active(prj, &name, active)?;
            switched.push((name, active));
        }
    }
    Ok(switched)
}

/// The id is a page or an item on a page.
pub fn in_pages(prj: &Project, id: u64) -> bool {
    prj.pages.iter().any(|p| p.id == id || item_ids(p).contains(&id))
}

/// Takes the objects out of the pages (whole pages with their content).
fn take(prj: &mut Project, ids: &BTreeSet<u64>) -> Vec<Parked> {
    let mut out = Vec::new();
    let mut before = 0;
    let mut kept = Vec::new();
    for (index, pg) in std::mem::take(&mut prj.pages).into_iter().enumerate() {
        let id = pg.id;
        if ids.contains(&id) {
            out.push(Parked { page: before, index, item: Item::Page(Box::new(pg)) });
        } else {
            kept.push(pg);
        }
        before = id;
    }
    prj.pages = kept;
    for pg in &mut prj.pages {
        let page = pg.id;
        macro_rules! take_list {
            ($list:ident, $variant:ident, $wrap:expr) => {
                let mut i = 0;
                pg.$list.retain(|x| {
                    let keep = !ids.contains(&x.id);
                    if !keep {
                        out.push(Parked { page, index: i, item: Item::$variant($wrap(x.clone())) });
                    }
                    i += 1;
                    keep
                });
            };
        }
        take_list!(symbols, Symbol, Box::new);
        take_list!(wires, Wire, Box::new);
        take_list!(texts, Text, std::convert::identity);
        take_list!(shapes, Shape, std::convert::identity);
        take_list!(dimensions, Dimension, std::convert::identity);
        take_list!(images, Image, std::convert::identity);
        take_list!(conn_defs, ConnDef, std::convert::identity);
        take_list!(cable_defs, CableDef, std::convert::identity);
    }
    out
}

/// Puts parked content back (pages first, items in the order they were taken out).
/// Items whose page no longer exists are dropped.
fn put_back(prj: &mut Project, parked: Vec<Parked>) {
    let (pages, items): (Vec<Parked>, Vec<Parked>) = parked.into_iter().partition(|p| matches!(p.item, Item::Page(_)));
    for p in pages {
        let Item::Page(pg) = p.item else { continue };
        let at = if p.page == 0 {
            0
        } else {
            prj.pages.iter().position(|x| x.id == p.page).map(|i| i + 1).unwrap_or(p.index.min(prj.pages.len()))
        };
        prj.pages.insert(at, *pg);
    }
    for p in items {
        let Some(pg) = prj.pages.iter_mut().find(|x| x.id == p.page) else { continue };
        fn put<T>(list: &mut Vec<T>, index: usize, x: T) {
            list.insert(index.min(list.len()), x);
        }
        match p.item {
            Item::Page(_) => {}
            Item::Symbol(x) => put(&mut pg.symbols, p.index, *x),
            Item::Wire(x) => put(&mut pg.wires, p.index, *x),
            Item::Text(x) => put(&mut pg.texts, p.index, x),
            Item::Shape(x) => put(&mut pg.shapes, p.index, x),
            Item::Dimension(x) => put(&mut pg.dimensions, p.index, x),
            Item::Image(x) => put(&mut pg.images, p.index, x),
            Item::ConnDef(x) => put(&mut pg.conn_defs, p.index, x),
            Item::CableDef(x) => put(&mut pg.cable_defs, p.index, x),
        }
    }
}

/// Number of members of an option that are pages.
pub fn page_count(prj: &Project, o: &ModelOption) -> usize {
    let parked = o.parked.iter().filter(|p| matches!(p.item, Item::Page(_))).count();
    parked + prj.pages.iter().filter(|p| o.members.contains(&p.id)).count()
}

#[cfg(test)]
mod tests;
