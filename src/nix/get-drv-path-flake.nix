{ attrPath, system }:
flake:
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

  inPackages =
    if system != null && flake ? packages.${system} then
      getAttrByPath attrPath flake.packages.${system}
    else
      "missing";
  inLegacyPackages =
    if system != null && flake ? legacyPackages.${system} then
      getAttrByPath attrPath flake.legacyPackages.${system}
    else
      "missing";
  inRoot = getAttrByPath attrPath flake;
in
if inPackages ? ok then
  getDrvPath inPackages.ok
else if inLegacyPackages ? ok then
  getDrvPath inLegacyPackages.ok
else if inRoot ? ok then
  getDrvPath inRoot.ok
else
  "missing"
