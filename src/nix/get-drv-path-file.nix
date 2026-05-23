{
  repoRoot,
  pathInRepo,
  rev,
  attrPathJson,
}:
let
  getAttrByPath =
    attrPath: set:
    let
      numAttrs = builtins.length attrPath;
      getAttrByPath' =
        n: v:
        let
          attr = builtins.elemAt attrPath n;
        in
        if n == numAttrs then
          { ok = v; }
        else if v ? ${attr} then
          getAttrByPath' (n + 1) v.${attr}
        else
          "missing";
    in
    getAttrByPath' 0 set;

  getDrvPath =
    v: if (v.type or null) == "derivation" then { ok = v.drvPath; } else { unexpectedType = typeOf v; };

  typeOf =
    v:
    builtins.typeOf v
    + (
      if builtins.isString (v.type or null) then
        " (with type = ${v.type})"
      else if builtins.isString (v._type or null) then
        " (with _type = ${v._type})"
      else
        ""
    );

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

  evalRoot = autoApply (import path);
  selected = getAttrByPath (builtins.fromJSON attrPathJson) evalRoot;
in
if selected ? ok then getDrvPath selected.ok else selected
