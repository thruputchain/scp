# shellcheck shell=bash


source ./scripts/setup_cargo.sh

echo 'setting up ccache'
ln -s "$USER_HOME/.cache" "$HOME/.cache"

echo 'cargo version check'
cargo --version

# The shared rustup install does not always carry it; ci.toml runs `cargo fmt`.
echo 'rustfmt component'
rustup component add rustfmt

echo 'cargo fetch'
CARGO_NET_GIT_FETCH_WITH_CLI=true cargo fetch
