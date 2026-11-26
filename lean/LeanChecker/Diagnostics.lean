import LeanChecker.Model
import Lean

open Lean
namespace LeanChecker

def collectDiagnostics (snap : Language.SnapshotTree) : IO (Array DiagItem × Bool) := do
  let allMsgs := snap.getAll.map (·.diagnostics.msgLog) |>.foldl (· ++ ·) MessageLog.empty
  let mut diags : Array DiagItem := #[]
  for m in allMsgs.toList do
    let sev := match m.severity with
      | .error => "error"
      | .warning => "warning"
      | .information => "info"
    let endPos := m.endPos.getD m.pos
    diags := diags.push {
      severity := sev
      file := m.fileName
      line := m.pos.line
      col := m.pos.column + 1
      endLine := endPos.line
      endCol := endPos.column + 1
      message := ← m.data.toString
    }
  return (diags, allMsgs.hasErrors)

end LeanChecker
