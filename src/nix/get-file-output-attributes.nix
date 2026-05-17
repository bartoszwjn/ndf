{ path }:
let
  autoApply = x: if builtins.isFunction x then x { } else x;
in
builtins.attrNames (autoApply (import path))
