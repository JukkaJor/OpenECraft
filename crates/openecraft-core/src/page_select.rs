//! Page selection for output (print, PDF): page names and ranges in project order,
//! e.g. `1-3, 5, =A1/7`.

use crate::model::Project;
use crate::structure::page_full_name;
use openecraft_i18n::tr_fmt;

fn find(prj: &Project, name: &str) -> Option<usize> {
    let name = name.trim();
    prj.pages.iter().position(|pg| pg.name == name).or_else(|| prj.pages.iter().position(|pg| page_full_name(pg) == name))
}

/// The indices of the pages a selection names, in project order without duplicates;
/// an empty selection is all pages. A range `a-b` runs from page `a` to page `b` in
/// project order (names that contain `-` themselves are found as a whole first).
pub fn parse(prj: &Project, spec: &str) -> Result<Vec<usize>, String> {
    if spec.trim().is_empty() {
        return Ok((0..prj.pages.len()).collect());
    }
    let mut on = vec![false; prj.pages.len()];
    for item in spec.split([',', ';']).map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(i) = find(prj, item) {
            on[i] = true;
            continue;
        }
        let range = item.match_indices('-').find_map(|(k, _)| Some((find(prj, &item[..k])?, find(prj, &item[k + 1..])?)));
        match range {
            Some((a, b)) => on[a.min(b)..=a.max(b)].iter_mut().for_each(|x| *x = true),
            None => return Err(tr_fmt!("No page named {}", item)),
        }
    }
    Ok((0..on.len()).filter(|i| on[*i]).collect())
}

#[cfg(test)]
mod tests;
