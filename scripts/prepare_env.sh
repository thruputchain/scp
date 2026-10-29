# shellcheck shell=bash


source ./scripts/setup_cargo.sh

echo 'setting up ccache'
ln -s "$USER_HOME/.cache" "$HOME/.cache"

echo 'cargo version check'
cargo --version

# The shared rustup install does not always carry them; ci.toml runs
# `cargo fmt` and `cargo clippy`.
echo 'rustfmt and clippy components'
rustup component add rustfmt clippy

echo 'cargo fetch'
cargo fetch
