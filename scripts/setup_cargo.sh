set -x
echo 'setting up cargo'
rm -r .cargo/ # clear the .cargo directory, so that we can symlink into the actual .cargo install

# curl -sSf https://sh.rustup.rs | sh -s -- -y
ln -s "$USER_HOME/.cargo" "$HOME/.cargo"
ln -s "$USER_HOME/.rustup" "$HOME/.rustup"
source "$HOME/.cargo/env"

rustup toolchain list
