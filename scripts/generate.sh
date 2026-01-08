#!/bin/bash

# XDR files in protocol/ from which to generate rust
protos=(
  scp.x
  scp_config.x
  scp_control.x
  scp_handle.x
)

[[ -z $XDRC ]] && XDRC="../../hogd/build/x23/x23c/x23c"

echo "-- Using XDRC=${XDRC} --" >&2

generate() {
  proto=${1:?}
  package=${2:?}
  name=$(basename "${proto}" .x)
  $XDRC --rust -o "${package}/src/${name}.rs" "${proto}"
}

for proto in "${protos[@]}"; do
  generate "protocol/${proto}" "scp_proto"
done
