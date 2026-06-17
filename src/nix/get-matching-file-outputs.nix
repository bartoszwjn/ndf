{
  repoRoot,
  pathInRepo,
  rev,
  queriesJson,
}:
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
in
map (getMatching evalRoot) queries
