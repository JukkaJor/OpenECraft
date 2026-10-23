//! A small XML reader for catalog files (BMEcat): elements with their local names
//! (namespace prefixes dropped), attributes, children and text. Comments, processing
//! instructions and the DOCTYPE are skipped; entities and CDATA are resolved.

/// An XML element.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Element {
    pub name: String,
    pub attributes: Vec<(String, String)>,
    pub children: Vec<Element>,
    /// The text directly inside the element (children's text not included).
    pub text: String,
}

impl Element {
    /// The first child with this name.
    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|c| c.name == name)
    }

    /// All children with this name.
    pub fn all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> + 'a {
        self.children.iter().filter(move |c| c.name == name)
    }

    /// Trimmed text of the first child with this name ("" if there is none).
    pub fn text_of(&self, name: &str) -> &str {
        self.child(name).map(|c| c.text.trim()).unwrap_or("")
    }

    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes.iter().find(|a| a.0 == name).map(|a| a.1.as_str())
    }

    /// All descendants with this name, depth first.
    pub fn descendants<'a>(&'a self, name: &str, out: &mut Vec<&'a Element>) {
        for c in &self.children {
            if c.name == name {
                out.push(c);
            }
            c.descendants(name, out);
        }
    }
}

fn local(name: &str) -> String {
    name.rsplit(':').next().unwrap_or(name).to_string()
}

/// Resolves `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;` and character references.
pub fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let Some(end) = after.find(';').filter(|e| *e <= 10) else {
            out.push('&');
            rest = after;
            continue;
        };
        let entity = &after[..end];
        let ch = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            e if e.starts_with("#x") || e.starts_with("#X") => u32::from_str_radix(&e[2..], 16).ok().and_then(char::from_u32),
            e if e.starts_with('#') => e[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &after[end + 1..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn attributes(s: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = s.trim();
    while let Some(eq) = rest.find('=') {
        let name = rest[..eq].trim();
        let after = rest[eq + 1..].trim_start();
        let Some(q) = after.chars().next().filter(|c| *c == '"' || *c == '\'') else { break };
        let Some(end) = after[1..].find(q) else { break };
        out.push((local(name), unescape(&after[1..1 + end])));
        rest = after[end + 2..].trim_start();
    }
    out
}

/// Parses a document and returns its root element.
pub fn parse(text: &str) -> Result<Element, String> {
    let s = text.trim_start_matches('\u{feff}');
    let mut stack: Vec<Element> = vec![Element::default()];
    let mut i = 0;
    while i < s.len() {
        let rest = &s[i..];
        if !rest.starts_with('<') {
            let end = rest.find('<').unwrap_or(rest.len());
            if let Some(top) = stack.last_mut() {
                top.text.push_str(&unescape(&rest[..end]));
            }
            i += end;
            continue;
        }
        if let Some(r) = rest.strip_prefix("<!--") {
            i += 4 + r.find("-->").ok_or("unterminated comment")? + 3;
        } else if let Some(r) = rest.strip_prefix("<![CDATA[") {
            let end = r.find("]]>").ok_or("unterminated CDATA")?;
            if let Some(top) = stack.last_mut() {
                top.text.push_str(&r[..end]);
            }
            i += 9 + end + 3;
        } else if rest.starts_with("<?") {
            i += rest.find("?>").ok_or("unterminated processing instruction")? + 2;
        } else if rest.starts_with("<!") {
            // DOCTYPE, possibly with an internal subset in brackets.
            let end = match (rest.find('['), rest.find('>')) {
                (Some(b), Some(g)) if b < g => rest.find("]>").map(|e| e + 2),
                (_, Some(g)) => Some(g + 1),
                _ => None,
            };
            i += end.ok_or("unterminated declaration")?;
        } else if let Some(r) = rest.strip_prefix("</") {
            let end = r.find('>').ok_or("unterminated end tag")?;
            let name = local(r[..end].trim());
            let el = stack.pop().ok_or("unexpected end tag")?;
            if el.name != name {
                return Err(format!("end tag {name} does not close {}", el.name));
            }
            stack.last_mut().ok_or("unexpected end tag")?.children.push(el);
            i += 2 + end + 1;
        } else {
            // Start tag; '>' inside attribute values is not expected in catalogs.
            let end = rest.find('>').ok_or("unterminated start tag")?;
            let inner = &rest[1..end];
            let empty = inner.ends_with('/');
            let inner = inner.trim_end_matches('/');
            let (name, attrs) = match inner.find(|c: char| c.is_whitespace()) {
                Some(k) => (&inner[..k], &inner[k..]),
                None => (inner, ""),
            };
            let el = Element { name: local(name), attributes: attributes(attrs), ..Default::default() };
            if empty {
                stack.last_mut().ok_or("unexpected tag")?.children.push(el);
            } else {
                stack.push(el);
            }
            i += end + 1;
        }
    }
    if stack.len() != 1 {
        return Err(format!("element {} is not closed", stack.last().map(|e| e.name.as_str()).unwrap_or("")));
    }
    stack.pop().and_then(|doc| doc.children.into_iter().next()).ok_or_else(|| "no root element".to_string())
}

#[cfg(test)]
mod tests;
