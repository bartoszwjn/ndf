{
  repoRoot,
  pathInRepo,
  rev,
  attrPathJson,
}:
let
  inherit (builtins)
    attrNames
    concatStringsSep
    elemAt
    fetchGit
    fromJSON
    genList
    isAttrs
    isFunction
    length
    typeOf
    ;

  autoApply = x: if isFunction x then x { } else x;

  repoPath = /. + repoRoot;
  repo =
    if rev == null then
      repoPath
    else
      fetchGit {
        url = repoPath;
        inherit rev;
      };
  path = if pathInRepo == "" then repo else repo + "/${pathInRepo}";

  getDrvByPath =
    attrPath: set:
    let
      numAttrs = length attrPath;
      showPath = concatStringsSep ".";
      sublist = len: list: genList (n: elemAt list n) len;

      notFound =
        n: context:
        throw (
          "attribute '${elemAt attrPath n}' in selection path '${showPath attrPath}' not found"
          + " inside path '${showPath (sublist n attrPath)}', "
          + context
        );

      showNames =
        set:
        let
          names = attrNames set;
          numNames = length names;
          maxShown = 10;
          numShown = if maxShown < numNames then maxShown else numNames;
          numNotShown = numNames - numShown;
        in
        concatStringsSep ", " (sublist numShown names)
        + (if 0 < numNotShown then " and ${toString numNotShown} more" else "");

      getDrvByPath' =
        n: v:
        let
          attr = elemAt attrPath n;
        in
        if n == numAttrs then
          v.drvPath
        else if !(isAttrs v) then
          notFound n "which is not an attribute set, but a ${typeOf v}"
        else if !(v ? ${attr}) then
          notFound n "which contains names ${showNames v}"
        else
          getDrvByPath' (n + 1) v.${attr};
    in
    getDrvByPath' 0 set;
in
getDrvByPath (fromJSON attrPathJson) (autoApply (import path))
