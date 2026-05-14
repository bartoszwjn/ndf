# List available recipes
default:
    just --list

# Update the expected outputs of integration tests based on actual outputs.
update-tests *test_names:
    NDF_TESTS_UPDATE=1 cargo test --test integration -- --exact {{ test_names }}

# Prepare all example directories
make-examples *args:
    cargo xtask examples make {{ args }}

# Clean up all example directories
clean-examples *args:
    cargo xtask examples clean {{ args }}
