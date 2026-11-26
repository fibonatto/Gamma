import LeanChecker.Model
import LeanChecker.Declarations
import LeanChecker.Diagnostics
import LeanChecker.Context
import Lean

open Lean Elab Command Meta
namespace LeanChecker

def inspectFile (filePath : System.FilePath) (importedMods : Array Name) : IO InspectOutput := do
  unsafe enableInitializersExecution
  initSearchPath (← findSysroot)
  let input ← IO.FS.readFile filePath
  let inputCtx := Parser.mkInputContext input filePath.toString
  let mainModuleName := Name.mkSimple (filePath.fileStem.getD "Main")
  let opts := Options.empty
  let opts := Lean.internal.cmdlineSnapshots.setIfNotSet opts true
  let opts := Elab.async.setIfNotSet opts true
  let setup stx := do
    return .ok {
      imports := stx.imports
      isModule := stx.isModule
      mainModuleName
      opts
      trustLevel := 0
      plugins := #[]
    }
  let snap ← (Language.Lean.process setup none).run ⟨inputCtx⟩
  let snaps := Language.toSnapshotTree snap
  let (diags, hasErrors) ← collectDiagnostics snaps
  let some cmdState := Language.Lean.waitForFinalCmdState? snap
    | return { diagnostics := diags, sections := #[], activeVars := #[], hasErrors }
  let cmdCtx : Command.Context := {
    cmdPos := ⟨0⟩
    fileName := filePath.toString
    fileMap := inputCtx.fileMap
    snap? := none
    cancelTk? := none
  }
  let inspectAction : CommandElabM (Array DeclSection × Array String) := do
    let env ← getEnv
    let importedBuckets := collectAllImportedInfos env importedMods
    let localInfos ← liftIO <| env.getLocalConstantInfos (skipTheoremSubDecls := true)
    let localConsts := localInfos.map AsyncConstantInfo.toConstantInfo
    let mut propDefs : NameSet := {}
    let addProps (infos : Array ConstantInfo) (pd : NameSet) : NameSet := Id.run do
      let mut acc := pd
      for info in infos do
        unless isInternal info.name do
          if let .defnInfo val := info then
            if val.type == Expr.sort Level.zero && val.safety != .partial then
              acc := acc.insert info.name
      return acc
    for (_, infos) in importedBuckets do
      propDefs := addProps infos propDefs
    propDefs := addProps localConsts propDefs
    let mut sections := #[]
    for (modLabel, infos) in importedBuckets do
      sections := sections.push (← collectFromInfos modLabel propDefs infos)
    sections := sections.push (← collectFromInfos filePath.toString propDefs localConsts)
    let activeVars ← runTermElabM fun xs => do
      let mut vars := #[]
      for x in xs do
        let nameFmt ← ppExpr x
        let typeFmt ← ppExpr (← inferType x)
        vars := vars.push (renderActiveVar nameFmt.pretty typeFmt.pretty)
      pure vars
    return (sections, activeVars)
  match (← EIO.toIO' <| (inspectAction cmdCtx).run cmdState) with
  | .error e => throw <| IO.Error.userError s!"internal error: {← e.toMessageData.toString}"
  | .ok ((sections, activeVars), _) =>
    return { diagnostics := diags, sections, activeVars, hasErrors }

def parseModuleNames (csv : String) : Array Name := Id.run do
  let mut out := #[]
  for item in csv.splitOn "," do
    let item := item.trimAscii.toString
    unless item.isEmpty do
      out := out.push (item.splitOn "." |>.foldl Name.mkStr Name.anonymous)
  return out

@[export gamma_inspect]
def gammaInspect (path modulesCsv : String) : IO String := do
  let result ← inspectFile (System.FilePath.mk path) (parseModuleNames modulesCsv)
  return (toJson result).compress

end LeanChecker
