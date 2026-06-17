{
  queriesJson,
  nixos,
  system,
}:
flake:
let
  queries = builtins.fromJSON queriesJson;

  getMatching =
    root: query:
    let
      pathLen = builtins.length query;
      getMatching' =
        node: path:
        let
          level = builtins.length path;
          q = builtins.elemAt query level;
          keys = if builtins.isAttrs node then builtins.attrNames node else [ ];
          recurse = key: getMatching' node.${key} (path ++ [ key ]);
        in
        if level == pathLen then
          [ path ]
        else if q.regex then
          builtins.concatMap (key: if builtins.match q.value key != null then recurse key else [ ]) keys
        else if node ? ${q.value} then
          recurse q.value
        else
          [ ];
    in
    getMatching' root [ ];
in
map (
  query:
  if query.leadingDot then
    getMatching flake query.path
  else if nixos then
    getMatching (flake.nixosConfigurations or { }) query.path
  else
    let
      inPackages = getMatching (flake.packages.${system} or { }) query.path;
      inLegacyPackages = getMatching (flake.legacyPackages.${system} or { }) query.path;
      inRoot = getMatching flake query.path;
    in
    if builtins.length inPackages != 0 then
      inPackages
    else if builtins.length inLegacyPackages != 0 then
      inLegacyPackages
    else
      inRoot
) queries
