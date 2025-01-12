# Justfile for the Rust Geodesy project.

alias l := list
alias r := run
alias t := test
alias tt := test-all
alias rr := run-all

# Harmless default
default: list

# list all justfile targets
list:
    just -l

# Basic test: Just library unit tests. Use target "test-all" or "check" for successively more in depth check ups.
test:
    cargo test --lib

# Unit tests, doc tests, pile test, and compiling of examples
test-all:
    cargo test

# Check that all tests pass, and that formatting and coding conventions are OK.
check:
    cargo clippy
    cargo fmt -- --check
    cargo test
    cargo doc --no-deps
    cargo package --allow-dirty
    git status

# Clean, then check
clean-check:
    cargo clean
    just check

# Tree of modules and data types
tree:
    cargo modules generate tree --lib --with-types

# Build and install assets
#assets:
#    zip -r assets.zip assets
#    mv assets.zip $LOCALAPPDATA/geodesy
#    ls -l $LOCALAPPDATA/geodesy

# Build documentation, open in browser for inspection.
doc:
    cargo doc --no-deps --open

# Run default application.
run ARGS:
    cargo run -- {{ARGS}}


# Run pq application.
pq ARGS:
    cargo run --bin=pq -- {{ARGS}}

# Run example based on its unique prefix (e.g. 00, 01, etc.).
run-example EXAMPLE:
    cargo run --example `basename examples/"{{EXAMPLE}}"* .rs`

# Run default application and all examples.
run-all:
    cargo run -- --help
    cargo run --example 00-transformations
    cargo run --example 01-geometric_geodesy
    cargo run --example 02-user_defined_macros
    cargo run --example 03-user_defined_operators

# Compact format log for changelog report
changes:
    git log --pretty=format:"%as: %s (%an)"
    # git log --pretty=format:"%as: %s (%an)" > CHANGELOG

# Some invisible oddities for general amusement

_sysinfo:
    @echo "This is an {{arch()}} machine, running {{os()}}".

_python:
    #!env python
    print('Hello from python!')
