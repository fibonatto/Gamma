use serde::Deserialize;

#[derive(Deserialize)]
pub struct DeclSection {
    #[serde(rename = "moduleName")] pub module_name: String,
    pub theorems: Vec<String>, pub ctors: Vec<String>,
    #[serde(rename = "openGoals")] pub open_goals: Vec<String>,
    pub axioms: Vec<String>, pub defs: Vec<String>,
}
#[derive(Deserialize)]
pub struct DiagItem {
    pub severity: String, pub file: String, pub line: usize, pub col: usize,
    #[serde(rename = "endLine")] pub end_line: usize,
    #[serde(rename = "endCol")] pub end_col: usize,
    pub message: String,
}
#[derive(Deserialize)]
pub struct InspectOutput {
    pub diagnostics: Vec<DiagItem>, pub sections: Vec<DeclSection>,
    #[serde(rename = "activeVars")] pub active_vars: Vec<String>,
    #[serde(rename = "hasErrors")] pub has_errors: bool,
}
