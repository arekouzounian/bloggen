/*
Copyright © 2024 Arek Ouzounian <arek@arekouzounian.com>
*/
package cmd

import (
	"github.com/spf13/cobra"
)

// postCmd represents the post command
var postCmd = &cobra.Command{
	Use:   "post",
	Short: "For making and managing blog posts.",
	Long: `To create a new blog post, use the init command: 
	
	bloggen post init <blog post name>
`,
}

func init() {
	rootCmd.AddCommand(postCmd)
}
