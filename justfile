# List available recipes
default:
    just --list

# Update the expected outputs of integration tests based on actual outputs.
update-integration-tests *test_names:
    NDF_TESTS_UPDATE=1 cargo test --test integration -- --exact {{ test_names }}

# Run integration tests with a CppNix binary in PATH
integration-tests-cppnix *args:
    nix shell --inputs-from . nixpkgs#nix --command cargo test --test integration {{ args }}

# Run integration tests with a Lix binary in PATH
integration-tests-lix *args:
    nix shell --inputs-from . nixpkgs#lix --command cargo test --test integration {{ args }}

# Prepare all example directories
make-examples *args:
    cargo xtask examples make {{ args }}

# Clean up all example directories
clean-examples *args:
    cargo xtask examples clean {{ args }}
