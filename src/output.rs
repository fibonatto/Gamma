use std::time::Duration;
use crate::diagnostics::InspectOutput;

pub fn print_output(out: &InspectOutput, elapsed: Duration) {
    for sec in &out.sections {
        if sec.theorems.is_empty() && sec.ctors.is_empty() && sec.open_goals.is_empty() && sec.axioms.is_empty() && sec.defs.is_empty() { continue; }
        println!("\n\x1b[1;36m══ {} ══\x1b[0m", sec.module_name);
        for (label, items) in [("THEOREMS", &sec.theorems), ("CONSTRUCTORS", &sec.ctors), ("OPEN GOALS (sorry)", &sec.open_goals), ("AXIOMS / OPAQUE", &sec.axioms), ("DEFINITIONS", &sec.defs)] {
            if !items.is_empty() { println!("\x1b[1;35m── {label} ──\x1b[0m"); for item in items { println!("{item}"); } }
        }
    }
    if !out.active_vars.is_empty() { println!("\n\x1b[1;35m── ACTIVE VARIABLES ──\x1b[0m"); for v in &out.active_vars { println!("{v}"); } }
    if !out.diagnostics.is_empty() {
        println!("\n\x1b[1m── DIAGNOSTICS ──\x1b[0m");
        for d in &out.diagnostics { let color = match d.severity.as_str() { "error" => "31", "warning" => "33", _ => "36" }; println!("\x1b[1;{color}m{}:{}:{}: {}:\x1b[0m {}", d.file, d.line, d.col, d.severity, d.message); }
    }
    println!("\n\x1b[2mgamma {:.3}s\x1b[0m", elapsed.as_secs_f64());
}
