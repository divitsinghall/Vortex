// Package main is the entry point for the vortex CLI.
//
// Build with: go build -o vortex .
// Run with: ./vortex --help
package main

import (
	"os"

	"github.com/vortex/vortex-cli/cmd"
)

func main() {
	if err := cmd.Execute(); err != nil {
		os.Exit(1)
	}
}
