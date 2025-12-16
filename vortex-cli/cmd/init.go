package cmd

import (
	"os"

	"github.com/spf13/cobra"
)

// Sample JavaScript code for new functions
const sampleCode = `// Welcome to Vortex!
//
// This is a sample serverless function.
// Modify it and deploy with: vortex deploy index.js

console.log("Hello from Vortex! 🚀");

// Perform some computation
const result = {
    message: "Hello, World!",
    timestamp: new Date().toISOString(),
    numbers: Array.from({ length: 5 }, (_, i) => i * i),
};

console.log("Result:", JSON.stringify(result, null, 2));

// Return the result to the caller
Vortex.return(result);
`

// initCmd represents the init command
var initCmd = &cobra.Command{
	Use:   "init",
	Short: "Initialize a new Vortex function",
	Long: `Creates a sample index.js file in the current directory.
This file contains a starter template for your serverless function.`,
	Run: runInit,
}

func init() {
	rootCmd.AddCommand(initCmd)
}

func runInit(cmd *cobra.Command, args []string) {
	printBanner()
	printInfo("Initializing new Vortex function...")

	filename := "index.js"

	// Check if file already exists
	if fileExists(filename) {
		printError("File %s already exists. Refusing to overwrite.", filename)
		printInfo("Use a different directory or remove the existing file.")
		os.Exit(1)
	}

	// Write sample code
	err := os.WriteFile(filename, []byte(sampleCode), 0644)
	checkError(err, "Failed to create file")

	printSuccess("Created %s", filename)
	printInfo("Next steps:")
	dimPrint("  1. Edit %s to add your logic\n", filename)
	dimPrint("  2. Deploy with: vortex deploy %s\n", filename)
	dimPrint("  3. Run with: vortex run <function_id>\n")
}
