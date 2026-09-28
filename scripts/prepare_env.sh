# shellcheck shell=bash


source ./scripts/setup_cargo.sh

echo 'setting up ccache'
ln -s "$USER_HOME/.cache" "$HOME/.cache"

echo 'cargo version check'
cargo --version

echo 'cargo fetch'
CARGO_NET_GIT_FETCH_WITH_CLI=true cargo fetch
