package go_scp

// XDR man page: https://xdrpp.github.io/goxdr/pkg/github.com/xdrpp/goxdr/cmd/goxdr/goxdr.1.html

//go:generate goxdr -o generated.go -p go_scp /Users/petar/scp/scp/scripts/../protocol/scp_config.x /Users/petar/scp/scp/scripts/../protocol/scp_control.x /Users/petar/scp/scp/scripts/../protocol/scp_handle.x /Users/petar/scp/scp/scripts/../protocol/scp_types.x /Users/petar/scp/scp/scripts/../protocol/scp.x 
