use std::{
    collections::HashMap,
    fs,
    io::{self, BufWriter, Write},
    time::Duration,
};

use crate::diagnostics::InspectOutput;

pub struct RunStats {
    pub wall: Duration,
    pub graph_time: Duration,
    pub inspect_time: Duration,
    pub user_cpu: Duration,
    pub sys_cpu: Duration,
    pub peak_rss_bytes: u64,
    pub cache_hit: bool,
    pub module_count: usize,
}

#[inline]
fn is_sym_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '\''
}

#[inline]
fn is_math_op(c: char) -> bool {
    matches!(
        c,
        '∀' | '∃'
            | '→'
            | '↔'
            | '∧'
            | '∨'
            | '¬'
            | '⊢'
            | '≤'
            | '≥'
            | '≠'
            | '∣'
            | '×'
            | '∑'
            | '∏'
            | '∘'
            | '⟨'
            | '⟩'
            | '↦'
    )
}

#[inline]
fn push_ansi(out: &mut String, code: &str, text: &str) {
    out.push_str(code);
    out.push_str(text);
    out.push_str("\x1b[0m");
}

fn highlight_lean_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 64);

    for (li, line) in text.split('\n').enumerate() {
        if li > 0 {
            out.push('\n');
        }
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        let mut at_line_start = true;
        let mut expect_decl_name = false;

        while i < chars.len() {
            let ch = chars[i];
            if ch.is_whitespace() {
                out.push(ch);
                i += 1;
                continue;
            }

            if expect_decl_name && ch != ':' {
                let mut j = i;
                while j < chars.len() && !chars[j].is_whitespace() && chars[j] != ':' {
                    j += 1;
                }
                let name: String = chars[i..j].iter().collect();
                push_ansi(&mut out, "\x1b[1;34m", &name);
                expect_decl_name = false;
                i = j;
                continue;
            }
            expect_decl_name = false;

            if at_line_start && ch == '⊢' {
                push_ansi(&mut out, "\x1b[1;33m", "⊢");
                at_line_start = false;
                i += 1;
                continue;
            }

            if ch == '"' {
                let mut j = i + 1;
                while j < chars.len() {
                    if chars[j] == '\\' && j + 1 < chars.len() {
                        j += 2;
                    } else if chars[j] == '"' {
                        j += 1;
                        break;
                    } else {
                        j += 1;
                    }
                }
                let lit: String = chars[i..j].iter().collect();
                push_ansi(&mut out, "\x1b[33m", &lit);
                at_line_start = false;
                i = j;
                continue;
            }

            if is_sym_char(ch) {
                let mut j = i + 1;
                while j < chars.len() && is_sym_char(chars[j]) {
                    j += 1;
                }
                let tok: String = chars[i..j].iter().collect();

                if at_line_start
                    && matches!(
                        tok.as_str(),
                        "theorem"
                            | "lemma"
                            | "ctor"
                            | "def"
                            | "abbrev"
                            | "axiom"
                            | "opaque"
                            | "variable"
                            | "inductive"
                            | "structure"
                            | "instance"
                            | "class"
                    )
                {
                    push_ansi(&mut out, "\x1b[1;35m", &tok);
                    expect_decl_name = true;
                } else if at_line_start && tok == "sorry" {
                    push_ansi(&mut out, "\x1b[1;33m", &tok);
                    expect_decl_name = true;
                } else if matches!(
                    tok.as_str(),
                    "forall"
                        | "exists"
                        | "fun"
                        | "let"
                        | "in"
                        | "if"
                        | "then"
                        | "else"
                        | "match"
                        | "with"
                        | "by"
                        | "where"
                        | "have"
                        | "show"
                        | "do"
                        | "return"
                        | "sorry"
                ) {
                    push_ansi(&mut out, "\x1b[35m", &tok);
                } else if matches!(
                    tok.as_str(),
                    "Nat"
                        | "Int"
                        | "Bool"
                        | "Prop"
                        | "Type"
                        | "Sort"
                        | "String"
                        | "List"
                        | "Option"
                        | "True"
                        | "False"
                        | "Fin"
                        | "Unit"
                        | "Array"
                        | "IO"
                        | "Char"
                        | "Float"
                ) {
                    push_ansi(&mut out, "\x1b[32m", &tok);
                } else if tok.bytes().all(|b| b.is_ascii_digit()) {
                    push_ansi(&mut out, "\x1b[33m", &tok);
                } else {
                    out.push_str(&tok);
                }
                at_line_start = false;
                i = j;
            } else {
                at_line_start = false;
                if is_math_op(ch) {
                    let mut buf = [0u8; 4];
                    push_ansi(&mut out, "\x1b[36m", ch.encode_utf8(&mut buf));
                } else {
                    out.push(ch);
                }
                i += 1;
            }
        }
    }

    out
}

fn write_code_snippet(
    w: &mut impl Write,
    file_cache: &mut HashMap<String, Vec<String>>,
    file_path: &str,
    start_line: usize,
    start_col: usize,
    end_line: usize,
    end_col: usize,
    is_error: bool,
) -> io::Result<()> {
    if !file_cache.contains_key(file_path) {
        let lines = fs::read_to_string(file_path)
            .map(|s| s.lines().map(str::to_owned).collect())
            .unwrap_or_default();
        file_cache.insert(file_path.to_owned(), lines);
    }

    let Some(lines) = file_cache.get(file_path) else {
        return Ok(());
    };
    if start_line == 0 || start_line > lines.len() {
        return Ok(());
    }

    let end_l = end_line.min(lines.len()).max(start_line);
    let width = end_l.to_string().len();
    let color = if is_error { "\x1b[31m" } else { "\x1b[33m" };

    for idx in (start_line - 1)..end_l {
        let line = &lines[idx];
        let chars: Vec<char> = line.chars().collect();
        let sc = start_col.saturating_sub(1).min(chars.len());
        let ec = end_col.saturating_sub(1).max(sc + 1).min(chars.len());

        if idx == start_line - 1 && idx == end_l - 1 {
            let prefix: String = chars[..sc].iter().collect();
            let span: String = chars[sc..ec].iter().collect();
            let suffix: String = chars[ec..].iter().collect();
            writeln!(
                w,
                "\x1b[2m{:>width$} | {prefix}\x1b[0m{color}\x1b[4m{span}\x1b[0m\x1b[2m{suffix}\x1b[0m",
                idx + 1,
                width = width
            )?;
        } else {
            writeln!(
                w,
                "\x1b[2m{:>width$} | \x1b[0m{color}{line}\x1b[0m",
                idx + 1,
                width = width
            )?;
        }
    }

    Ok(())
}

fn format_wall(d: Duration) -> String {
    let secs = d.as_secs_f64();
    if secs >= 60.0 {
        let mins = (secs / 60.0).floor() as u64;
        let rem = secs - (mins as f64 * 60.0);
        format!("{mins}:{rem:05.2}")
    } else {
        format!("{secs:.3}s")
    }
}

fn write_output(w: &mut impl Write, out: &InspectOutput, stats: &RunStats) -> io::Result<()> {
    for sec in &out.sections {
        if sec.theorems.is_empty()
            && sec.ctors.is_empty()
            && sec.open_goals.is_empty()
            && sec.axioms.is_empty()
            && sec.defs.is_empty()
        {
            continue;
        }
        writeln!(w, "\n\x1b[1;36m══ {} ══\x1b[0m", sec.module_name)?;
        for (label, items) in [
            ("THEOREMS", &sec.theorems),
            ("CONSTRUCTORS", &sec.ctors),
            ("OPEN GOALS (sorry)", &sec.open_goals),
            ("AXIOMS / OPAQUE", &sec.axioms),
            ("DEFINITIONS", &sec.defs),
        ] {
            if !items.is_empty() {
                writeln!(w, "\x1b[1;35m── {label} ──\x1b[0m")?;
                for item in items {
                    writeln!(w, "{}", highlight_lean_text(item))?;
                }
            }
        }
    }

    if !out.active_vars.is_empty() {
        writeln!(w, "\n\x1b[1;35m── ACTIVE VARIABLES ──\x1b[0m")?;
        for v in &out.active_vars {
            writeln!(w, "{}", highlight_lean_text(v))?;
        }
    }

    if !out.diagnostics.is_empty() {
        writeln!(w, "\n\x1b[1m── DIAGNOSTICS ──\x1b[0m")?;
        let mut file_cache = HashMap::new();
        for d in &out.diagnostics {
            let color = match d.severity.as_str() {
                "error" => "31",
                "warning" => "33",
                _ => "36",
            };
            writeln!(
                w,
                "\x1b[1;{color}m{}:{}:{}: {}:\x1b[0m {}",
                d.file,
                d.line,
                d.col,
                d.severity,
                highlight_lean_text(&d.message)
            )?;
            if d.severity != "info" {
                write_code_snippet(
                    w,
                    &mut file_cache,
                    &d.file,
                    d.line,
                    d.col,
                    d.end_line,
                    d.end_col,
                    d.severity == "error",
                )?;
            }
        }
    }

    let wall_s = stats.wall.as_secs_f64();
    let user_s = stats.user_cpu.as_secs_f64();
    let sys_s = stats.sys_cpu.as_secs_f64();
    let cpu_pct = if wall_s > 0.0 {
        ((user_s + sys_s) / wall_s * 100.0).round() as u64
    } else {
        0
    };
    let rss_mib = stats.peak_rss_bytes as f64 / (1024.0 * 1024.0);
    let wall_str = format_wall(stats.wall);
    let cache_label = if stats.cache_hit {
        "cache hit"
    } else {
        "cache miss"
    };
    let inspect_label = if stats.cache_hit { "cache" } else { "lean" };

    writeln!(
        w,
        "\n\x1b[2mgamma     {user_s:.3}s user {sys_s:.3}s system {cpu_pct}% cpu {wall_str} total | rss {rss_mib:.1} MiB | {} modules ({cache_label})\x1b[0m",
        stats.module_count
    )?;
    writeln!(
        w,
        "\x1b[2mphases    graph {:.1}ms | {inspect_label} {:.1}ms\x1b[0m",
        stats.graph_time.as_secs_f64() * 1000.0,
        stats.inspect_time.as_secs_f64() * 1000.0
    )?;

    if out.has_errors {
        writeln!(w, "\x1b[1;31m[gamma] Failed (exit 1, {wall_str})\x1b[0m")?;
    } else {
        writeln!(w, "\x1b[1;32m[gamma] Passed (exit 0, {wall_str})\x1b[0m")?;
    }

    Ok(())
}

pub fn print_output(out: &InspectOutput, stats: &RunStats) {
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    let _ = write_output(&mut writer, out, stats);
    let _ = writer.flush();
}
