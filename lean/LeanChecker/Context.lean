import Lean

namespace LeanChecker

def renderActiveVar (name ty : String) : String :=
  s!"variable {name} : {ty}"

end LeanChecker
