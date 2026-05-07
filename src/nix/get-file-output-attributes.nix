{
  repoRoot,
  pathInRepo,
  rev,
}:
let
  autoApply = x: if builtins.isFunction x then x { } else x;

  repo =
    if rev == null then
      repoRoot
    else
      builtins.fetchGit {
        url = /. + repoRoot;
        inherit rev;
      };
  path = if pathInRepo == "" then repo else repo + "/${pathInRepo}";
in
builtins.attrNames (autoApply (import path))
