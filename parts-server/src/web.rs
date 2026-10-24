//! Web page of the data portal (search with facets, part details, files, favorites)
//! and its translations. The page is plain HTML / JavaScript on top of the API; its
//! texts are written in English and translated through [`texts`].

use axum::extract::Query;
use axum::response::Html;
use axum::Json;
use openecraft_i18n::{tr_const, translate, Lang};
use openecraft_parts::portal::{AttachmentKind, ChangeKind, Lifecycle, Role, Sort};
use openecraft_parts::{productgroups, TopGroup, PRODUCT_GROUPS};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const PAGE: &str = include_str!("web/portal.html");

/// Texts of the page (`{}`: placeholders filled in by the page).
const TEXTS: &[&str] = &[
    tr_const!("OpenECraft data portal"),
    tr_const!("Search parts"),
    tr_const!("Search"),
    tr_const!("Sign in"),
    tr_const!("Sign out"),
    tr_const!("User name"),
    tr_const!("Password"),
    tr_const!("Signed in as {}"),
    tr_const!("Favorites"),
    tr_const!("All parts"),
    tr_const!("Catalogs"),
    tr_const!("Language"),
    tr_const!("Product top group"),
    tr_const!("Product group"),
    tr_const!("Product subgroup"),
    tr_const!("Manufacturer"),
    tr_const!("Catalog"),
    tr_const!("Lifecycle"),
    tr_const!("Technical data"),
    tr_const!("Clear filters"),
    tr_const!("{} parts"),
    tr_const!("No parts found"),
    tr_const!("Part number"),
    tr_const!("Description"),
    tr_const!("Price"),
    tr_const!("Sort by"),
    tr_const!("Descending"),
    tr_const!("Previous"),
    tr_const!("Next"),
    tr_const!("Page {} of {}"),
    tr_const!("Back to the results"),
    tr_const!("Type number"),
    tr_const!("Order number"),
    tr_const!("Supplier"),
    tr_const!("Note"),
    tr_const!("Identifier"),
    tr_const!("Voltage"),
    tr_const!("Current"),
    tr_const!("Power"),
    tr_const!("Certificates"),
    tr_const!("Datasheet"),
    tr_const!("Width"),
    tr_const!("Height"),
    tr_const!("Depth"),
    tr_const!("Weight"),
    tr_const!("Mounting"),
    tr_const!("Functions"),
    tr_const!("Accessories"),
    tr_const!("Successor"),
    tr_const!("Replaced by {}"),
    tr_const!("Files"),
    tr_const!("History"),
    tr_const!("Revision {}"),
    tr_const!("Last change"),
    tr_const!("Add to favorites"),
    tr_const!("Remove from favorites"),
    tr_const!("Sign in to keep favorites"),
    tr_const!("Version"),
    tr_const!("Parts"),
    tr_const!("Show parts"),
    tr_const!("Export (CSV)"),
    tr_const!("Loading…"),
    tr_const!("The server cannot be reached"),
    tr_const!("General"),
    tr_const!("Commercial data"),
    tr_const!("Mounting data"),
    tr_const!("Has 3D model"),
    tr_const!("Has 2D model"),
    tr_const!("{} parts in {} catalogs, revision {}"),
];

#[derive(Deserialize)]
pub struct TextQuery {
    lang: Option<String>,
}

/// Translations for the page: `texts` by English text, `keys` by `kind:Key` of the
/// enumerations (`top_group:Fluid`, `lifecycle:Active`, `kind:Datasheet`, …).
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WebTexts {
    pub lang: String,
    /// All languages: code and native name.
    pub languages: Vec<(String, String)>,
    pub texts: BTreeMap<String, String>,
    pub keys: BTreeMap<String, String>,
}

pub async fn index() -> Html<&'static str> {
    Html(PAGE)
}

pub fn web_texts(lang: Lang) -> WebTexts {
    let own_groups = PRODUCT_GROUPS.iter().flat_map(|(g, subs)| std::iter::once(*g).chain(subs.iter().copied()));
    let group_names = productgroups::GROUPS.iter().chain(productgroups::SUBGROUPS).map(|(_, n)| *n).chain(own_groups);
    let texts = TEXTS.iter().copied().chain(group_names).map(|t| (t.to_string(), translate(t, lang).to_string())).collect();
    let mut keys = BTreeMap::new();
    let mut key = |kind: &str, k: String, title: &'static str| {
        keys.insert(format!("{kind}:{k}"), translate(title, lang).to_string());
    };
    for t in TopGroup::ALL {
        key("top_group", format!("{t:?}"), t.title());
    }
    for l in Lifecycle::ALL {
        key("lifecycle", format!("{l:?}"), l.title());
    }
    for k in AttachmentKind::ALL {
        key("kind", format!("{k:?}"), k.title());
    }
    for s in Sort::ALL {
        key("sort", format!("{s:?}"), s.title());
    }
    for r in Role::ALL {
        key("role", format!("{r:?}"), r.title());
    }
    for c in [ChangeKind::Created, ChangeKind::Changed, ChangeKind::Deleted] {
        key("change", format!("{c:?}"), c.title());
    }
    let languages = Lang::ALL.iter().map(|l| (l.code().to_string(), l.native_name().to_string())).collect();
    WebTexts { lang: lang.code().into(), languages, texts, keys }
}

/// `GET /api/v1/texts?lang=de`: the page's texts in a language (English if unknown).
pub async fn texts(Query(q): Query<TextQuery>) -> Json<WebTexts> {
    let lang = q.lang.as_deref().and_then(Lang::from_code).unwrap_or(Lang::En);
    Json(web_texts(lang))
}

#[cfg(test)]
mod tests;
