/*
Copyright © 2024 Arek Ouzounian <arek@arekouzounian.com>
*/
package cmd

import (
	"os"

	"github.com/spf13/cobra"
)

// rootCmd represents the base command when called without any subcommands
var rootCmd = &cobra.Command{
	Use:   "bloggen",
	Short: "A blogging framework tool",
	Long: `This is the CLI frontend for the bloggen client. 
This tool is designed to interface with the server, as well as provide extra utilities, such as file conversion.`,
}

func Execute() {
	err := rootCmd.Execute()
	if err != nil {
		os.Exit(1)
	}
}

func init() {

}
