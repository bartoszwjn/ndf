{
  repoRoot,
  rev,
}:
builtins.fetchGit {
  url = /. + repoRoot;
  inherit rev;
}
