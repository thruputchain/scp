#!/bin/sh

set -xeuo pipefail

SCRIPT_DIR=$(dirname $(readlink -f "$0"))

# generate pkg.go

X_FILES=$(ls -1 $SCRIPT_DIR/../protocol/*.x | tr '\n' ' ')
PKG_NAME="go_scp"

cat <<EOF > $SCRIPT_DIR/../go_scp/pkg.go
package $PKG_NAME

// XDR man page: https://xdrpp.github.io/goxdr/pkg/github.com/xdrpp/goxdr/cmd/goxdr/goxdr.1.html

//go:generate goxdr -o generated.go -p $PKG_NAME $X_FILES
EOF

# build go

cd $SCRIPT_DIR/../go_scp

go mod tidy
go generate
go build
