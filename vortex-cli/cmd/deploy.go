package cmd

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"time"

	"github.com/spf13/cobra"
)

// DeployRequest matches the API's expected format
type DeployRequest struct {
	Code string `json:"code"`
}

// DeployResponse matches the API's response format
type DeployResponse struct {
	FunctionID string `json:"function_id"`
}

// deployCmd represents the deploy command
var deployCmd = &cobra.Command{
	Use:   "deploy <filename>",
	Short: "Deploy a JavaScript function to Vortex",
	Long: `Uploads a JavaScript file to the Vortex platform.
Returns a unique function_id that can be used to execute the function.

Example:
  vortex deploy index.js
  vortex deploy ./functions/handler.js`,
	Args: cobra.ExactArgs(1),
	Run:  runDeploy,
}

func init() {
	rootCmd.AddCommand(deployCmd)
}

func runDeploy(cmd *cobra.Command, args []string) {
	filename := args[0]

	// Validate file exists
	if !fileExists(filename) {
		fatal("File not found: %s", filename)
	}

	// Read file contents
	code, err := os.ReadFile(filename)
	checkError(err, "Failed to read file")

	if len(code) == 0 {
		fatal("File is empty: %s", filename)
	}

	printInfo("Deploying %s (%d bytes)...", filepath.Base(filename), len(code))

	// Prepare request
	reqBody := DeployRequest{Code: string(code)}
	jsonBody, err := json.Marshal(reqBody)
	checkError(err, "Failed to encode request")

	// Send request
	client := &http.Client{Timeout: 30 * time.Second}
	resp, err := client.Post(
		apiURL+"/deploy",
		"application/json",
		bytes.NewReader(jsonBody),
	)
	checkError(err, "Failed to connect to API")
	defer resp.Body.Close()

	// Read response
	body, err := io.ReadAll(resp.Body)
	checkError(err, "Failed to read response")

	// Handle errors
	if resp.StatusCode != http.StatusCreated && resp.StatusCode != http.StatusOK {
		var errResp map[string]interface{}
		if json.Unmarshal(body, &errResp) == nil {
			if msg, ok := errResp["error"].(string); ok {
				fatal("Deploy failed: %s", msg)
			}
		}
		fatal("Deploy failed with status %d: %s", resp.StatusCode, string(body))
	}

	// Parse success response
	var deployResp DeployResponse
	err = json.Unmarshal(body, &deployResp)
	checkError(err, "Failed to parse response")

	// Print success
	printSuccess("Function deployed successfully!")
	fmt.Println()
	infoPrint("Function ID: ")
	fmt.Println(deployResp.FunctionID)
	fmt.Println()
	dimPrint("Run your function with:\n")
	dimPrint("  vortex run %s\n", deployResp.FunctionID)
}
