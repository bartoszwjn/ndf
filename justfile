# List available recipes
default:
    just --list

# Prepare all example directories
make-examples *args:
    cargo xtask examples make {{ args }}

# Clean up all example directories
clean-examples *args:
    cargo xtask examples clean {{ args }}
