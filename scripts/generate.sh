#!/bin/bash

set -xeuo pipefail

SCRIPT_DIR=$(dirname $(readlink -f "$0"))

# XDR files in protocol/ from which to generate rust
protos=(
  scp.x
  scp_config.x
  scp_control.x
  scp_handle.x
  scp_types.x
)

[[ -z $XDRC ]] && XDRC="../../hogd/vscode/x23/x23c/x23c"

echo "-- Using XDRC=${XDRC} --" >&2

generate() {
  proto=${1:?}
  package=${2:?}
  name=$(basename "${proto}" .x)
  $XDRC --rust --rust-feature xdrs --rust-feature serde --rust-no-feature spec -o "${package}/src/${name}.rs" "${proto}"
}

for proto in "${protos[@]}"; do
  generate "protocol/${proto}" "scp_proto"
done

$SCRIPT_DIR/go_generate.sh
