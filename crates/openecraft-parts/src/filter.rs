//! Part filter of the part trees: free text plus manufacturer, supplier and product
//! group, and the criterion the tree is grouped by.

use crate::{natural_cmp, Part, TopGroup};

/// Criterion of the first tree level.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Grouping {
    /// Top group › product group › subgroup.
    #[default]
    ProductGroup,
    Manufacturer,
    Supplier,
}

impl Grouping {
    pub const ALL: [Grouping; 3] = [Grouping::ProductGroup, Grouping::Manufacturer, Grouping::Supplier];

    pub fn title(self) -> &'static str {
        match self {
            Grouping::ProductGroup => "Product group",
            Grouping::Manufacturer => "Manufacturer",
            Grouping::Supplier => "Supplier",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PartFilter {
    /// Searched in number, type, order number, manufacturer, description and groups.
    pub text: String,
    pub manufacturer: Option<String>,
    pub supplier: Option<String>,
    pub top_group: Option<TopGroup>,
    pub group: Option<String>,
    pub grouping: Grouping,
}

impl PartFilter {
    pub fn matches(&self, p: &Part) -> bool {
        let eq = |want: &Option<String>, have: &str| want.as_deref().is_none_or(|w| w == have);
        p.matches_text(&self.text)
            && eq(&self.manufacturer, &p.manufacturer)
            && eq(&self.supplier, &p.supplier)
            && self.top_group.is_none_or(|t| t == p.top_group)
            && eq(&self.group, &p.group)
    }

    /// True if any criterion restricts the parts (the grouping does not).
    pub fn is_active(&self) -> bool {
        !self.text.is_empty() || self.manufacturer.is_some() || self.supplier.is_some() || self.top_group.is_some() || self.group.is_some()
    }

    /// Clears all criteria, keeps the grouping.
    pub fn reset(&mut self) {
        *self = PartFilter { grouping: self.grouping, ..Default::default() };
    }
}

/// Distinct non-empty values of a part field, naturally sorted.
pub fn distinct_values<'a>(parts: impl IntoIterator<Item = &'a Part>, field: impl Fn(&Part) -> &str) -> Vec<String> {
    let mut values: Vec<String> = parts.into_iter().map(&field).filter(|v| !v.is_empty()).map(String::from).collect();
    values.sort_by(|a, b| natural_cmp(a, b));
    values.dedup();
    values
}

#[cfg(test)]
mod tests;
