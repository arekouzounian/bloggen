/*
Copyright © 2023 NAME HERE <EMAIL ADDRESS>
*/
package cmd

import (
	"fmt"
	"log"
	"os"

	// "net"

	"github.com/spf13/cobra"
	"golang.org/x/crypto/ssh"
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
		port := args[0]

		server, err := cmd.Flags().GetString("target")
		if err != nil {
			fmt.Println("Error getting target flag")
			return
		}

		key, err := os.ReadFile("/Users/arekouzounian/.ssh/id_rsa")
		if err != nil {
			log.Fatalf("unable to read private key: %v", err)
		}

		// Create the Signer for this private key.
		signer, err := ssh.ParsePrivateKey(key)
		if err != nil {
			log.Fatalf("unable to parse private key: %v", err)
		}

		config := &ssh.ClientConfig{
			Auth: []ssh.AuthMethod{
				ssh.PublicKeys(signer),
			},
			HostKeyCallback: ssh.InsecureIgnoreHostKey(),
		}

		// Connect to SSH server
		conn, err := ssh.Dial("tcp", fmt.Sprintf("%s:%s", server, port), config)
		if err != nil {
			log.Fatalf("Failed to connect to SSH server: %v", err)
		}
		defer conn.Close()

		// Create new SSH session
		session, err := conn.NewSession()
		if err != nil {
			log.Fatalf("Failed to create SSH session: %v", err)
		}
		defer session.Close()

		in, err := session.StdinPipe()
		if err != nil {
			log.Fatalf("Failed to create stdin pipe: %v", err)
		}

		fmt.Fprintf(in, "asdfasdfasdfas")
	},
}

// func callback(hostname string, remote net.Addr, key ssh.PublicKey) error {

// 	// ip_addr := strings.Split(remote.String(), ":")[0]

// 	// if ip_addr != "127.0.0.1" {
// 	// 	return errors.New("FUCK YOU!!!! LOCALHOST ONLY")
// 	// }
// 	return nil
// }

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
