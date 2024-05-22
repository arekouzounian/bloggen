/*
Copyright © 2024 Arek Ouzounian <arek@arekouzounian.com>
*/
package cmd

import (
	"fmt"
	"log"
	"net"
	"os"

	"github.com/pkg/sftp"
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
			HostKeyCallback: ssh.InsecureIgnoreHostKey(), // change this later
		}

		// Connect to SSH server
		conn, err := ssh.Dial("tcp", fmt.Sprintf("%s:%s", server, port), config)
		if err != nil {
			log.Fatalf("Failed to connect to SSH server: %v", err)
		}
		defer conn.Close()

		sftpClient, err := sftp.NewClient(conn)
		if err != nil {
			log.Fatalf("Failed to create sftp client: %v", err)
		}
		defer sftpClient.Close() // for some reason this hangs

		fmt.Println("sftp client connected.")

		// file, err := sftpClient.OpenFile("asdf.txt", os.O_CREATE|os.O_WRONLY|os.O_TRUNC)
		// if err != nil {
		// 	log.Fatalf("%v", err)
		// }

		// file.Write([]byte("FUCKER THIS WORKED!!"))

		// err = file.Close()
		// if err != nil {
		// 	log.Fatalf("%v", err)
		// }

		err = sftpClient.Mkdir("/test")      // should fail
		err2 := sftpClient.Mkdir("../test3") // should fail

		fmt.Printf("%t\n", err != nil && err2 != nil)

		err3 := sftpClient.Mkdir("test/")      // should work
		err4 := sftpClient.Mkdir("test/asdf/") // should work
		err5 := sftpClient.Mkdir("test2")      // should work

		fmt.Printf("%t\n", err3 == err4 && err4 == err5) // should all be nil
	},
}

func callback(hostname string, remote net.Addr, key ssh.PublicKey) error {

	// ip_addr := strings.Split(remote.String(), ":")[0]

	// if ip_addr != "127.0.0.1" {
	// 	return errors.New("FUCK YOU!!!! LOCALHOST ONLY")
	// }
	// parse known hosts

	return nil
}

func init() {
	rootCmd.AddCommand(testCmd)
	testCmd.Flags().StringP("target", "t", "localhost", "The target domain or IP address to connect to. Default is localhost")
}
