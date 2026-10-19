use std::path::{Path, PathBuf};

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(rust_files(&path));
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    out
}

fn is_test_file(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()) == Some("tests.rs")
}

/// Unit tests live in `src/<module>/tests.rs`, never inline in the module file.
#[test]
fn unit_tests_are_in_separate_files() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();
    for path in rust_files(&src).into_iter().filter(|p| !is_test_file(p)) {
        let text = std::fs::read_to_string(&path).unwrap();
        let name = path.strip_prefix(&src).unwrap().display().to_string();
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            if t.starts_with("//") {
                continue;
            }
            if t.starts_with("#[test]") || t.starts_with("mod tests {") || t.starts_with("mod tests{") {
                violations.push(format!("{name}:{}: inline test code – move it to {}", i + 1, tests_path(&name)));
            }
            if t == "mod tests;" && (i == 0 || lines[i - 1].trim() != "#[cfg(test)]") {
                violations.push(format!("{name}:{}: `mod tests;` must be preceded by `#[cfg(test)]`", i + 1));
            }
        }
    }
    assert!(violations.is_empty(), "test layout violations (see AGENTS.md):\n{}", violations.join("\n"));
}

/// Every `tests.rs` belongs to a module file next to its directory (`foo.rs` + `foo/tests.rs`).
#[test]
fn test_files_belong_to_a_module() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut orphans = Vec::new();
    for path in rust_files(&src).into_iter().filter(|p| is_test_file(p)) {
        let dir = path.parent().unwrap();
        let module_file = dir.with_extension("rs");
        let declared = std::fs::read_to_string(&module_file)
            .map(|t| t.lines().any(|l| l.trim() == "mod tests;"))
            .unwrap_or(false);
        if !declared {
            orphans.push(format!(
                "{} is not declared via `#[cfg(test)] mod tests;` in {}",
                path.strip_prefix(&src).unwrap().display(),
                module_file.strip_prefix(&src).unwrap_or(&module_file).display()
            ));
        }
    }
    assert!(orphans.is_empty(), "{}", orphans.join("\n"));
}

fn tests_path(module_file: &str) -> String {
    format!("src/{}/tests.rs", module_file.trim_end_matches(".rs"))
}
