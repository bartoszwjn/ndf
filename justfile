# List available recipes
default:
    just --list

# Run Nix checks and integration tests
check:
    nix flake check --keep-going
    cargo test --test integration

# Run `cargo clippy`
clippy:
    cargo clippy --workspace --all-targets

# Update the expected outputs of integration tests based on actual outputs.
update-integration-tests *test_names:
    NDF_TESTS_UPDATE=1 cargo test --test integration -- --exact {{ test_names }}

# Run integration tests
integration-tests *args:
    cargo test --test integration {{ args }}

# Run integration tests with a CppNix binary in PATH
integration-tests-cppnix *args:
    nix shell --inputs-from . nixpkgs#nix --command cargo test --test integration {{ args }}

# Run integration tests with a Lix binary in PATH
integration-tests-lix *args:
    nix shell --inputs-from . nixpkgs#lix --command cargo test --test integration {{ args }}
