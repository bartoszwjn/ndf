# `ndf`

A command-line program that makes it easier to compare
how a set of Nix derivations changed between commits.

The motivating use case is having a Git repository with many NixOS configurations,
with parts of the configuration factored out into common modules.
`ndf` allows you to check how a given change to Nix code
affected each configuration at the derivation level (see [Worry-free NixOS refactors]).
By default `ndf` shows only whether a derivation changed at all,
but it can also show how the derivations differ by using [`nix-diff`].

## Installation

Use the Nix flake in this repository to install the package into a profile,
add it to a NixOS or Home Manager configuration,
or run it directly from the command line:

```bash
nix run github:bartoszwjn/ndf
```

`cargo install --git` should work as well.

The program expects the following commands to be available in `PATH` at runtime:

- `nix`, as well as `nix-instantiate`, `nix-build`, etc.
- `git`, if operating on a pure Git repository.
- `jj`, if operating on a colocated Jujutsu/Git repository.
- The external tool used to compare derivations, if specified using the `--tool` option.

## Usage

Run `ndf` to compare all `packages` outputs of the Nix flake in the current directory.
Use `--nixos` to compare `nixosConfigurations` instead.
Use `--flake` to choose the flake to compare (must be a Git worktree on the local filesystem),
`--file` to compare output attributes of a Nix expression stored in a file.
`--revision`, `--from` and `--to` can be used to select commits that are compared to each other.
Use positional arguments to manually specify which output attributes to compare.
When the `--glob`/`-g` flag is used,
positional arguments are treated as glob patterns
and matched against the existing output attributes of the flake or file being compared.

See the `--help` output for details about all command line flags and options.

`ndf` has special support for [Jujutsu workspaces][jj].
When `ndf` detects that the given repository is a Jujutsu workspace it switches to "Jujutsu mode",
in which revisions are specified using Jujutsu's [revset language][jj-revsets]
and `jj log` is used to display them.
The Jujutsu workspace must be a [colocated Jujutsu/Git workspace][jj-colocated-workspaces],
since Nix does not integrate with Jujutsu directly.
The automatic mode selection can be overridden using the `--git` and `--jj` flags.

[Worry-free NixOS refactors]: https://www.tweag.io/blog/2022-10-11-stable-narhashes/
[`nix-diff`]: https://github.com/Gabriella439/nix-diff/
[jj-colocated-workspaces]: https://www.jj-vcs.dev/latest/glossary/#colocated-workspaces
[jj-revsets]: https://www.jj-vcs.dev/latest/revsets/
[jj]: https://www.jj-vcs.dev
