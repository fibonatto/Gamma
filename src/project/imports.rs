/// Reads Lean's initial import header while respecting nested block comments and line comments.
/// This is intentionally a header scanner, not a complete Lean parser.
pub fn parse_imports(src: &str) -> Vec<String> {
    let bytes = src.as_bytes();
    let mut i = 0usize;
    let mut clean = Vec::with_capacity(bytes.len());
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            while i < bytes.len() && bytes[i] != b'\n' { clean.push(b' '); i += 1; }
        } else if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'-' {
            let mut depth = 1usize; clean.extend_from_slice(b"  "); i += 2;
            while i < bytes.len() && depth > 0 {
                if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'-' { depth += 1; clean.extend_from_slice(b"  "); i += 2; }
                else if i + 1 < bytes.len() && bytes[i] == b'-' && bytes[i + 1] == b'/' { depth -= 1; clean.extend_from_slice(b"  "); i += 2; }
                else { clean.push(if bytes[i] == b'\n' { b'\n' } else { b' ' }); i += 1; }
            }
        } else { clean.push(bytes[i]); i += 1; }
    }
    let text = String::from_utf8_lossy(&clean);
    let mut imports = Vec::new();
    for line in text.lines() {
        let mut line = line.trim();
        if line.is_empty() || line == "prelude" || line == "module" { continue; }
        if let Some(rest) = line.strip_prefix("public ") { line = rest.trim_start(); }
        if let Some(rest) = line.strip_prefix("import ") {
            imports.extend(rest.split_whitespace().map(str::to_owned));
        } else { break; }
    }
    imports
}

#[cfg(test)]
mod tests {
    use super::parse_imports;

    #[test]
    fn parses_imports_and_ignores_nested_comments() {
        let src = "prelude\n/- outer /- nested -/ done -/\nimport Alpha.Beta\nimport Gamma -- comment\ndef x := 1\nimport Ignored";
        assert_eq!(parse_imports(src), vec!["Alpha.Beta", "Gamma"]);
    }

    #[test]
    fn handles_public_import_and_utf8_comments() {
        let src = "-- comentário 日本語\npublic import Foo.Bar\nimport Baz\n#check Nat";
        assert_eq!(parse_imports(src), vec!["Foo.Bar", "Baz"]);
    }
}
