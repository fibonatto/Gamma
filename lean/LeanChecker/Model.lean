import Lean
import Lean.Data.Json

namespace LeanChecker

structure DeclSection where
  moduleName : String
  theorems   : Array String := #[]
  ctors      : Array String := #[]
  openGoals  : Array String := #[]
  axioms     : Array String := #[]
  defs       : Array String := #[]
  deriving Lean.ToJson

structure DiagItem where
  severity : String
  file     : String
  line     : Nat
  col      : Nat
  endLine  : Nat
  endCol   : Nat
  message  : String
  deriving Lean.ToJson

structure InspectOutput where
  diagnostics : Array DiagItem
  sections    : Array DeclSection
  activeVars  : Array String
  hasErrors   : Bool
  deriving Lean.ToJson

end LeanChecker
