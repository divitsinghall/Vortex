// Package cmd contains all CLI commands for vortex.
package cmd

import (
	"fmt"
	"os"

	"github.com/fatih/color"
	"github.com/spf13/cobra"
)

// Configuration defaults (can be overridden with flags)
var (
	apiURL string
)

// Color printers for consistent UX
var (
	successPrint = color.New(color.FgGreen, color.Bold).PrintfFunc()
	errorPrint   = color.New(color.FgRed, color.Bold).PrintfFunc()
	infoPrint    = color.New(color.FgCyan).PrintfFunc()
	dimPrint     = color.New(color.Faint).PrintfFunc()
)

// rootCmd represents the base command when called without any subcommands
var rootCmd = &cobra.Command{
	Use:   "vortex",
	Short: "Vortex CLI - Deploy and run serverless JavaScript functions",
	Long: `Vortex CLI is a command-line tool for the Vortex FaaS platform.

Deploy JavaScript functions to the cloud and execute them instantly.

Examples:
  vortex init                    # Create a sample function
  vortex deploy index.js         # Deploy a function
  vortex run <function_id>       # Execute a deployed function`,
}

// Execute adds all child commands to the root command and sets flags appropriately.
// This is called by main.main(). It only needs to happen once to the rootCmd.
func Execute() error {
	return rootCmd.Execute()
}

func init() {
	// Global flags
	rootCmd.PersistentFlags().StringVar(&apiURL, "api", "http://localhost:8080", "Vortex API URL")
}

// printSuccess prints a green success message
func printSuccess(format string, a ...interface{}) {
	successPrint("✓ "+format+"\n", a...)
}

// printError prints a red error message
func printError(format string, a ...interface{}) {
	errorPrint("✗ "+format+"\n", a...)
}

// printInfo prints a cyan info message
func printInfo(format string, a ...interface{}) {
	infoPrint("→ "+format+"\n", a...)
}

// fatal prints an error and exits
func fatal(format string, a ...interface{}) {
	printError(format, a...)
	os.Exit(1)
}

// checkError prints error if not nil and exits
func checkError(err error, context string) {
	if err != nil {
		fatal("%s: %v", context, err)
	}
}

// fileExists returns true if the file exists
func fileExists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}

// printBanner prints the Vortex banner
func printBanner() {
	banner := color.New(color.FgMagenta, color.Bold)
	banner.Println(`
 __     __         _            
 \ \   / /__  _ __| |_ _____  __
  \ \ / / _ \| '__| __/ _ \ \/ /
   \ V / (_) | |  | ||  __/>  < 
    \_/ \___/|_|   \__\___/_/\_\
`)
	fmt.Println()
}
