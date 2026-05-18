{ attrPath, system }:
flake:
let
  inherit (builtins)
    concatStringsSep
    elemAt
    isAttrs
    length
    ;

  getAttrByPath =
    path: set:
    let
      numAttrs = length path;
      getAttrByPath' =
        n: v:
        let
          attr = elemAt path n;
        in
        if n == numAttrs then
          { found = v; }
        else if isAttrs v && v ? ${attr} then
          getAttrByPath' (n + 1) v.${attr}
        else
          { };
    in
    getAttrByPath' 0 set;

  inPackages = getAttrByPath attrPath flake.packages.${system};
  inLegacyPackages = getAttrByPath attrPath flake.legacyPackages.${system};
  inRoot = getAttrByPath attrPath flake;

  showPath = concatStringsSep "." attrPath;
  notFound = throw (
    "flake does not provide attribute "
    + (
      if system == null then
        "'${showPath}'"
      else
        "'packages.${system}.${showPath}', 'legacyPackages.${system}.${showPath}' or '${showPath}'"
    )
  );
in
if system != null && flake ? packages.${system} && inPackages ? found then
  inPackages.found.drvPath
else if system != null && flake ? legacyPackages.${system} && inLegacyPackages ? found then
  inLegacyPackages.found.drvPath
else if inRoot ? found then
  inRoot.found.drvPath
else
  notFound
