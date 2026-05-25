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

The program expects `nix` (as well as `nix-instantiate`, `nix-build`, etc.)
and `git` commands to be available in `PATH` at runtime.
When using external tools to compare derivations (the `--tool` option)
those tools need to be available in `PATH` as well.

## Usage

Run `ndf` to compare all `packages` outputs of the Nix flake in the current directory.
Use `--nixos` to compare `nixosConfigurations` instead.
Use `--flake` to choose the flake to compare (must be a Git repository on the local filesystem),
`--file` to compare output attributes of a Nix expression stored in a file.
`--from` and `--to` can be used to select commits that are compared against each other.
Use positional arguments to manually specify which output attributes to compare.

See the `--help` output for details about all command line flags and options.

## Roadmap

Planned changes:

- Add `--glob` flag to allow using glob patterns in attribute paths.
- [Jujutsu] support: detect colocated Jujutsu/Git repositories, display commits using `jj log`,
  allow selecting commits using `jj`'s revset language.

[Jujutsu]: https://www.jj-vcs.dev
[Worry-free NixOS refactors]: https://www.tweag.io/blog/2022-10-11-stable-narhashes/
[`nix-diff`]: https://github.com/Gabriella439/nix-diff/
