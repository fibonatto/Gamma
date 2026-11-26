import LeanChecker.Model
import Lean

open Lean Elab Command Meta
namespace LeanChecker

def isInternal (name : Name) : Bool := Id.run do
  let s := name.toString
  let last := (s.splitOn ".").getLast!
  return last ∈ ["rec", "casesOn", "recOn", "brecOn", "binductionOn",
          "below", "ibelow", "noConfusion", "noConfusionType",
          "ctorIdx", "ctorElim", "ctorElimType", "elim",
          "inj", "injEq", "sizeOf_spec", "_f", "_unsafe_rec", "_sunfold"] ||
    last.startsWith "match_" || last.startsWith "eq_" || last.startsWith "proof_" ||
    s.contains ".brecOn." || s.contains ".below." || s.contains ".ibelow." || s.contains "._"

def unfoldProps (propDefs : NameSet) (e : Expr) : CoreM Expr := do
  let env ← getEnv
  Core.transform e fun sub => do
    match sub with
    | .const n lvls =>
      if propDefs.contains n then
        if let some ci := env.find? n then
          if let some val := ci.value? then
            return .visit (val.instantiateLevelParams ci.levelParams lvls)
      return .continue
    | _ => return .continue

def renderDecl (header : String) (fmt : Format) (maxW : Nat := 60) : String :=
  let single := fmt.pretty (width := maxW) (indent := 2) (column := header.length + 1)
  if single.contains '\n' || header.length + 1 + single.length > maxW then
    let multi := fmt.pretty (width := maxW) (indent := 2) (column := 2)
    s!"{header}\n  {multi}"
  else
    s!"{header} {single}"

def collectFromInfos (modLabel : String) (propDefs : NameSet) (infos : Array ConstantInfo) : CommandElabM DeclSection := do
  let mut sec : DeclSection := { moduleName := modLabel }
  for info in infos do
    unless isInternal info.name do
      match info with
      | .thmInfo val =>
        let msg ← liftTermElabM do
          let ty ← Core.betaReduce (← unfoldProps propDefs val.type)
          let fmt ← ppExpr ty
          let header := if val.value.hasSorry then s!"sorry {info.name} :" else s!"theorem {info.name} :"
          pure (renderDecl header fmt)
        if val.value.hasSorry then
          sec := { sec with openGoals := sec.openGoals.push msg }
        else
          sec := { sec with theorems := sec.theorems.push msg }
      | .ctorInfo val =>
        let msg ← liftTermElabM do
          let ty ← Core.betaReduce (← unfoldProps propDefs val.type)
          pure (renderDecl s!"ctor {info.name} :" (← ppExpr ty))
        sec := { sec with ctors := sec.ctors.push msg }
      | .defnInfo val =>
        unless propDefs.contains info.name do
          let last := (info.name.toString.splitOn ".").getLast!
          unless last.endsWith "Aux" || last.endsWith "_aux" do
            let msg ← liftTermElabM do
              pure (renderDecl s!"def {info.name} :" (← ppExpr val.type))
            sec := { sec with defs := sec.defs.push msg }
      | .axiomInfo val =>
        let msg ← liftTermElabM do
          let ty ← Core.betaReduce (← unfoldProps propDefs val.type)
          pure (renderDecl s!"axiom {info.name} :" (← ppExpr ty))
        sec := { sec with axioms := sec.axioms.push msg }
      | .opaqueInfo val =>
        let msg ← liftTermElabM do
          let ty ← Core.betaReduce (← unfoldProps propDefs val.type)
          pure (renderDecl s!"opaque {info.name} :" (← ppExpr ty))
        sec := { sec with axioms := sec.axioms.push msg }
      | _ => pure ()
  return sec

/-- Collect constants for requested modules in one pass over `map₁`. -/
def collectAllImportedInfos (env : Environment) (mods : Array Name) : Array (String × Array ConstantInfo) := Id.run do
  let modNames := env.allImportedModuleNames
  let mut idxToPos : Std.HashMap Nat Nat := {}
  let mut buckets : Array (Array ConstantInfo) := #[]
  for i in [:mods.size] do
    buckets := buckets.push #[]
    if let some modIdx := modNames.findIdx? (· == mods[i]!) then
      idxToPos := idxToPos.insert modIdx i
  for (name, ci) in env.constants.map₁.toList do
    if let some idx := env.getModuleIdxFor? name then
      if let some pos := idxToPos.get? idx.toNat then
        buckets := buckets.set! pos (buckets[pos]!.push ci)
  let mut out := #[]
  for i in [:mods.size] do
    out := out.push (mods[i]!.toString, buckets[i]!)
  return out

end LeanChecker
