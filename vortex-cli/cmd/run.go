package cmd

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"

	"github.com/fatih/color"
	"github.com/spf13/cobra"
)

// ExecuteResponse matches the API's response format
type ExecuteResponse struct {
	Output          interface{} `json:"output"`
	Logs            []LogEntry  `json:"logs"`
	ExecutionTimeMs uint64      `json:"execution_time_ms"`
}

// LogEntry represents a single log message from the function
type LogEntry struct {
	Level     string `json:"level"`
	Message   string `json:"message"`
	Timestamp string `json:"timestamp"`
}

// runCmd represents the run command
var runCmd = &cobra.Command{
	Use:   "run <function_id>",
	Short: "Execute a deployed function",
	Long: `Executes a function by its ID and displays the result.

The function's console output and return value will be displayed.

Example:
  vortex run abc123-def456-...`,
	Args: cobra.ExactArgs(1),
	Run:  runFunction,
}

func init() {
	rootCmd.AddCommand(runCmd)
}

func runFunction(cmd *cobra.Command, args []string) {
	functionID := args[0]

	printInfo("Executing function %s...", functionID)
	fmt.Println()

	// Send execute request
	client := &http.Client{Timeout: 60 * time.Second}
	resp, err := client.Post(
		apiURL+"/execute/"+functionID,
		"application/json",
		bytes.NewReader([]byte("{}")),
	)
	checkError(err, "Failed to connect to API")
	defer resp.Body.Close()

	// Read response
	body, err := io.ReadAll(resp.Body)
	checkError(err, "Failed to read response")

	// Handle errors
	if resp.StatusCode != http.StatusOK {
		var errResp map[string]interface{}
		if json.Unmarshal(body, &errResp) == nil {
			if msg, ok := errResp["error"].(string); ok {
				fatal("Execution failed: %s", msg)
			}
		}
		fatal("Execution failed with status %d: %s", resp.StatusCode, string(body))
	}

	// Parse response
	var execResp ExecuteResponse
	err = json.Unmarshal(body, &execResp)
	checkError(err, "Failed to parse response")

	// Display logs
	if len(execResp.Logs) > 0 {
		logHeader := color.New(color.FgYellow, color.Bold)
		logHeader.Println("📋 Console Output:")
		fmt.Println()

		for _, log := range execResp.Logs {
			logColor := color.New(color.Faint)
			switch log.Level {
			case "error":
				logColor = color.New(color.FgRed)
			case "warn":
				logColor = color.New(color.FgYellow)
			case "info":
				logColor = color.New(color.FgCyan)
			}
			logColor.Printf("  [%s] %s\n", log.Level, log.Message)
		}
		fmt.Println()
	}

	// Display output
	resultHeader := color.New(color.FgGreen, color.Bold)
	resultHeader.Println("📦 Return Value:")
	fmt.Println()

	// Pretty print the output
	if execResp.Output != nil {
		prettyOutput, err := json.MarshalIndent(execResp.Output, "  ", "  ")
		if err == nil {
			fmt.Printf("  %s\n", string(prettyOutput))
		} else {
			fmt.Printf("  %v\n", execResp.Output)
		}
	} else {
		dimPrint("  (no return value)\n")
	}
	fmt.Println()

	// Display execution time
	timeColor := color.New(color.Faint)
	timeColor.Printf("⏱  Executed in %dms\n", execResp.ExecutionTimeMs)
}
