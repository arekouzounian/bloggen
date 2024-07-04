/*
Copyright © 2024 Arek Ouzounian <arek@arekouzounian.com>
*/
package cmd

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/spf13/cobra"
)

const (
	DEFAULT_SERVER_ADDR = "localhost:2222"
	SERVER_ENV_KEY      = "BLOGGEN_SERVER"
	EMPTY_STR           = ""
)

var (
	Host      string
	Keypath   string
	Hostsfile string
)

// postCmd represents the post command
var postCmd = &cobra.Command{
	Use:   "post",
	Short: "For making and managing blog posts.",
	Long: `To create a new blog post, use the init command:

	bloggen post init <blog post name>
`,
	PersistentPreRunE: func(cmd *cobra.Command, args []string) error {
		fmt.Println("post called")
		var err error

		Host, err = cmd.Flags().GetString("server")
		if err != nil {
			fmt.Println("Fatal: host flag not found")
			return err
		}
		Keypath, err = cmd.Flags().GetString("keyfile")
		if err != nil {
			fmt.Println("Fatal: keyfile flag not found")
			return err
		}
		Hostsfile, err = cmd.Flags().GetString("hostfile")
		if err != nil {
			fmt.Println("Fatal: hostfile flag not found")
			return err
		}

		if Host == EMPTY_STR {
			if env_key, exists := os.LookupEnv(SERVER_ENV_KEY); exists {
				Host = env_key
			} else {
				Host = DEFAULT_SERVER_ADDR
			}
		}

		homedir, err := os.UserHomeDir()
		if err != nil {
			fmt.Printf("Error accessing default keyfile: %v", err)
			return err
		}
		if Keypath == EMPTY_STR {
			Keypath = filepath.Join(homedir, ".ssh", "id_rsa")
		}
		if Hostsfile == EMPTY_STR {
			Hostsfile = filepath.Join(homedir, ".ssh", "known_hosts")
		}

		return nil
	},
}

func init() {
	rootCmd.AddCommand(postCmd)

	postCmd.PersistentFlags().StringP("server", "s", "", "Specifies the target bloggen server to upload to. If not specified, and no BLOGGEN_SERVER environment variable is configured, defaults to 'localhost:2222'")
	postCmd.PersistentFlags().StringP("keyfile", "k", "", "Specifies the target key file to use for ssh authentication. If not specified, defaults to `~/.ssh/id_rsa`")
	postCmd.PersistentFlags().StringP("hostfile", "f", "", "Specifies the target known_hosts file to use for ssh authentication. If not specified, defaults to `~/.ssh/known_hosts`")
}
