/*
Copyright © 2023 NAME HERE <EMAIL ADDRESS>
*/
package cmd

import (
	"bufio"
	"fmt"
	"net"

	// "net"

	"github.com/spf13/cobra"
)

// testCmd represents the test command
var testCmd = &cobra.Command{
	Use:   "test [port]",
	Short: "Tests connection to server",
	Long:  `Connects to specified port and prints out what is being returned by server.`,
	Run: func(cmd *cobra.Command, args []string) {
		if len(args) < 1 {
			fmt.Println("Missing argument: port number")
			return
		}

		target, err := cmd.Flags().GetString("target")
		if err != nil {
			fmt.Println("Error getting target flag")
			return
		}

		host := target + ":" + args[0]
		conn, err := net.Dial("tcp", host)
		if err != nil {
			fmt.Printf("Error dialing connection at %s\n", host)
			return
		}

		reader := bufio.NewReader(conn)
		for {
			str, err := reader.ReadString('\n')
			if err != nil {
				fmt.Println("Connection shutdown")
				break
			}

			if len(str) > 0 {
				fmt.Println(str)
			}
		}
	},
}

func init() {
	rootCmd.AddCommand(testCmd)

	// Here you will define your flags and configuration settings.

	// Cobra supports Persistent Flags which will work for this command
	// and all subcommands, e.g.:
	// testCmd.PersistentFlags().String("foo", "", "A help for foo")

	// Cobra supports local flags which will only run when this command
	// is called directly, e.g.:
	// testCmd.Flags().BoolP("toggle", "t", false, "Help message for toggle")
	testCmd.Flags().StringP("target", "t", "localhost", "The target domain or IP address to connect to. Default is localhost")
}
