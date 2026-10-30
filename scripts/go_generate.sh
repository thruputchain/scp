#!/bin/bash

set -xeuo pipefail

SCRIPT_DIR=$(dirname "$(readlink -f "$0")")

# generate pkg.go

# Paths relative to go_scp/, so the generated header does not embed the
# absolute path of whoever ran this script.
X_FILES=$(cd "$SCRIPT_DIR/../go_scp" && ls -1 ../protocol/*.x | tr '\n' ' ')
PKG_NAME="go_scp"

cat <<EOF > "$SCRIPT_DIR/../go_scp/pkg.go"
package $PKG_NAME

// XDR man page: https://xdrpp.github.io/goxdr/pkg/github.com/xdrpp/goxdr/cmd/goxdr/goxdr.1.html

//go:generate goxdr -o generated.go -p $PKG_NAME $X_FILES
EOF

# build go

cd "$SCRIPT_DIR/../go_scp"

go mod tidy
go generate
go build
