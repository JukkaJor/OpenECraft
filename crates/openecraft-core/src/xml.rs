//! Minimal XML reader for the exchange formats (AutomationML, GSDML, ESI): elements,
//! attributes and text. Namespace prefixes are dropped, DTDs, comments and processing
//! instructions are skipped.

use std::fmt::Write;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Element {
    /// Local name (without namespace prefix).
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Element>,
    pub text: String,
}

impl Element {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|c| c.name == name)
    }

    pub fn children<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> + 'a {
        self.children.iter().filter(move |c| c.name == name)
    }

    /// All descendants with this name, depth first.
    pub fn descendants<'a>(&'a self, name: &str) -> Vec<&'a Element> {
        let mut v = Vec::new();
        for c in &self.children {
            if c.name == name {
                v.push(c);
            }
            v.extend(c.descendants(name));
        }
        v
    }
}

/// Escapes text for attributes and element content.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\n' => out.push_str("&#10;"),
            c => out.push(c),
        }
    }
    out
}

fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(end) = rest.find(';').filter(|e| *e < 12) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..end];
        let c = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ if entity.starts_with("#x") || entity.starts_with("#X") => u32::from_str_radix(&entity[2..], 16).ok().and_then(char::from_u32),
            _ if entity.starts_with('#') => entity[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match c {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn local(name: &str) -> String {
    name.rsplit(':').next().unwrap_or(name).to_string()
}

/// Parses a document and returns its root element.
pub fn parse(text: &str) -> Result<Element, String> {
    let s = text.trim_start_matches('\u{feff}');
    let mut stack: Vec<Element> = Vec::new();
    let mut root = None;
    let mut i = 0;
    let b = s.as_bytes();
    while i < s.len() {
        if b[i] != b'<' {
            let end = s[i..].find('<').map(|e| i + e).unwrap_or(s.len());
            if let Some(top) = stack.last_mut() {
                top.text.push_str(&unescape(&s[i..end]));
            }
            i = end;
            continue;
        }
        let rest = &s[i..];
        if rest.starts_with("<!--") {
            i += rest.find("-->").ok_or("unterminated comment")? + 3;
        } else if rest.starts_with("<![CDATA[") {
            let end = rest.find("]]>").ok_or("unterminated CDATA")?;
            if let Some(top) = stack.last_mut() {
                top.text.push_str(&rest[9..end]);
            }
            i += end + 3;
        } else if rest.starts_with("<?") {
            i += rest.find("?>").ok_or("unterminated processing instruction")? + 2;
        } else if rest.starts_with("<!") {
            // DOCTYPE, possibly with an internal subset.
            let end = match (rest.find('['), rest.find('>')) {
                (Some(o), Some(c)) if o < c => rest.find("]>").ok_or("unterminated DOCTYPE")? + 2,
                (_, Some(c)) => c + 1,
                _ => return Err("unterminated DOCTYPE".into()),
            };
            i += end;
        } else if let Some(r) = rest.strip_prefix("</") {
            let end = r.find('>').ok_or("unterminated end tag")?;
            let name = local(r[..end].trim());
            let el = stack.pop().ok_or_else(|| format!("unexpected </{name}>"))?;
            if el.name != name {
                return Err(format!("</{name}> does not close <{}>", el.name));
            }
            match stack.last_mut() {
                Some(parent) => parent.children.push(el),
                None => root = Some(el),
            }
            i += end + 3;
        } else {
            let (el, len, empty) = start_tag(&rest[1..])?;
            i += len + 1;
            if empty {
                match stack.last_mut() {
                    Some(parent) => parent.children.push(el),
                    None => root = Some(el),
                }
            } else {
                stack.push(el);
            }
        }
        if root.is_some() && stack.is_empty() {
            break;
        }
    }
    if let Some(el) = stack.last() {
        return Err(format!("<{}> is not closed", el.name));
    }
    root.ok_or_else(|| "no root element".into())
}

/// Parses `name attr="v" …>` or `…/>`. Returns the element, the consumed length
/// (including `>`) and whether the tag was empty.
fn start_tag(s: &str) -> Result<(Element, usize, bool), String> {
    let mut el = Element::default();
    let name_end = s.find(|c: char| c.is_whitespace() || c == '>' || c == '/').ok_or("unterminated tag")?;
    el.name = local(&s[..name_end]);
    let mut i = name_end;
    loop {
        let rest = &s[i..];
        let trimmed = rest.trim_start();
        i += rest.len() - trimmed.len();
        if trimmed.starts_with("/>") {
            return Ok((el, i + 2, true));
        }
        if trimmed.starts_with('>') {
            return Ok((el, i + 1, false));
        }
        if trimmed.is_empty() {
            return Err("unterminated tag".into());
        }
        let eq = trimmed.find('=').ok_or("attribute without value")?;
        let key = trimmed[..eq].trim();
        let after = trimmed[eq + 1..].trim_start();
        let quote = after.chars().next().filter(|c| *c == '"' || *c == '\'').ok_or("attribute value without quotes")?;
        let end = after[1..].find(quote).ok_or("unterminated attribute value")?;
        let value = unescape(&after[1..1 + end]);
        if !key.starts_with("xmlns") {
            el.attrs.push((local(key), value));
        }
        i += trimmed.len() - after.len() + end + 2;
    }
}

/// Writer with indentation for the exports.
#[derive(Default)]
pub struct Writer {
    pub out: String,
    depth: usize,
}

impl Writer {
    pub fn new() -> Writer {
        Writer { out: String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n"), depth: 0 }
    }

    fn tag(&mut self, name: &str, attrs: &[(&str, &str)], close: &str) {
        let _ = write!(self.out, "{:1$}<{name}", "", self.depth * 2);
        for (k, v) in attrs {
            let _ = write!(self.out, " {k}=\"{}\"", esc(v));
        }
        self.out.push_str(close);
        self.out.push('\n');
    }

    pub fn open(&mut self, name: &str, attrs: &[(&str, &str)]) {
        self.tag(name, attrs, ">");
        self.depth += 1;
    }

    pub fn empty(&mut self, name: &str, attrs: &[(&str, &str)]) {
        self.tag(name, attrs, " />");
    }

    pub fn close(&mut self, name: &str) {
        self.depth -= 1;
        let _ = writeln!(self.out, "{:1$}</{name}>", "", self.depth * 2);
    }

    pub fn text(&mut self, name: &str, attrs: &[(&str, &str)], text: &str) {
        let _ = write!(self.out, "{:1$}<{name}", "", self.depth * 2);
        for (k, v) in attrs {
            let _ = write!(self.out, " {k}=\"{}\"", esc(v));
        }
        let _ = writeln!(self.out, ">{}</{name}>", esc(text));
    }
}

#[cfg(test)]
mod tests;
