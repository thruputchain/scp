package go_scp

// XDR man page: https://xdrpp.github.io/goxdr/pkg/github.com/xdrpp/goxdr/cmd/goxdr/goxdr.1.html

//go:generate goxdr -o generated.go -p go_scp ../protocol/scp_config.x ../protocol/scp_control.x ../protocol/scp_handle.x ../protocol/scp_types.x ../protocol/scp.x
