/*
Copyright © 2024 Arek Ouzounian <arek@arekouzounian.com>
*/
package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

// postCmd represents the post command
var listCmd = &cobra.Command{
	Use:   "list",
	Short: "lists all blog posts ",
	Long: `To list all currently posted blog posts:
	bloggen post list
`,
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println(Host)
		fmt.Println(Keypath)
		fmt.Println(Hostsfile)
	},
}

func init() {
	postCmd.AddCommand(listCmd)
}
