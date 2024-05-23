/*
Copyright © 2024 Arek Ouzounian <arek@arekouzounian.com>
*/
package cmd

import (
	"fmt"
	"io/fs"
	"log"
	"os"
	"path/filepath"
	"strings"

	"github.com/pkg/sftp"
	"github.com/spf13/cobra"
	"golang.org/x/crypto/ssh"
)

const (
	DEFAULT_SERVER_ADDR = "localhost:2222"
	SERVER_ENV_KEY      = "BLOGGEN_SERVER"
)

// uploadCmd represents the upload command
var uploadCmd = &cobra.Command{
	Use:   "upload",
	Short: "Uploads the post from the target directory to the given server",
	Long: `Sends the given directory (default '.') to the bloggen server via sftp. 
The directory should be in canonical bloggen structure; you can create the proper structure using the 'bloggen post init' command. 
This command will automatically attempt to convert the contained markdown to HTML. If you want to stop this, you can use the --no-conv flag.`,
	Run: func(cmd *cobra.Command, args []string) {
		target, err := cmd.Flags().GetString("target")
		if err != nil {
			fmt.Println("Fatal: target flag not found")
			return
		}
		no_conv, err := cmd.Flags().GetBool("no-conv")
		if err != nil {
			fmt.Println("Fatal: no-conv flag not found")
			return
		}
		host, err := cmd.Flags().GetString("server")
		if err != nil {
			fmt.Println("Fatal: host flag not found")
			return
		}
		keypath, err := cmd.Flags().GetString("keyfile")
		if err != nil {
			fmt.Println("Fatal: keyfile flag not found")
			return
		}

		if host == "" {
			if env_key, exists := os.LookupEnv(SERVER_ENV_KEY); exists {
				host = env_key
			} else {
				host = DEFAULT_SERVER_ADDR
			}
		}
		if keypath == "" {
			homedir, err := os.UserHomeDir()
			if err != nil {
				fmt.Printf("Error accessing default keyfile: %v", err)
			}
			keypath = homedir + "/.ssh/id_rsa"
		}

		path, err := filepath.Abs(target)
		if err != nil {
			fmt.Printf("Not a valid directory: %s\n", target)
			return
		}
		target = path + "/"

		md_file, err := ValidateDirectoryStructure(target)
		if err != nil {
			fmt.Printf("Invalid directory structure: %v\n", err)
			return
		}

		if !no_conv {
			ind := strings.LastIndex(md_file, "/")
			if ind >= 0 {
				new_path := md_file[:strings.LastIndex(md_file, ".")] + ".html"
				ConvertMDToHTML(md_file, new_path, false)
			} else {
				fmt.Printf("Path to blog post is not absolute: %s\n", md_file)
				return
			}
		} else {
			// check that html file exists
			hasHtml, err := FileExtensionExists(target, "html")
			if err != nil {
				fmt.Printf("Unable to read target directory %s: %v", target, err)
				return
			}
			if !hasHtml {
				fmt.Printf("Specified target directory (%s) lacks an HTML file. Consider using `bloggen conv`, or removing the `no-conv` flag.\n", target)
				return
			}
		}

		err = UploadPost(target, host, keypath)
		if err != nil {
			fmt.Println(err.Error())
			return
		}

		fmt.Println("Post uploaded to server successfully.")
	},
}

func UploadPost(directory_path string, host string, keypath string) error {
	key, err := os.ReadFile(keypath)
	if err != nil {
		return fmt.Errorf("unable to read private key: %v", err)
	}

	// Create the Signer for this private key.
	signer, err := ssh.ParsePrivateKey(key)
	if err != nil {
		return fmt.Errorf("unable to parse private key: %v", err)
	}

	config := &ssh.ClientConfig{
		Auth: []ssh.AuthMethod{
			ssh.PublicKeys(signer),
		},
		HostKeyCallback: ssh.InsecureIgnoreHostKey(), // change this later
	}

	// Connect to SSH server
	conn, err := ssh.Dial("tcp", host, config)
	if err != nil {
		return fmt.Errorf("failed to connect to SSH server: %v", err)
	}
	defer conn.Close()

	sftpClient, err := sftp.NewClient(conn)
	if err != nil {
		log.Fatalf("Failed to create sftp client: %v", err)
	}
	defer sftpClient.Close() // for some reason this hangs

	before_slash := directory_path[:len(directory_path)-2]
	folder_name_ind := strings.LastIndex(before_slash, "/")
	remote_folder_name := directory_path[folder_name_ind+1:]

	err = sftpClient.Mkdir(remote_folder_name)
	if err != nil {
		return err
	}

	fsys := os.DirFS(directory_path)
	err = fs.WalkDir(fsys, ".", func(path string, entry fs.DirEntry, err error) error {
		if err != nil {
			return fs.SkipDir
		}
		if entry.Name() == "." {
			return nil // skip
		}

		if entry.IsDir() {
			err = sftpClient.Mkdir(remote_folder_name + path)
			if err != nil {
				fmt.Printf("Error creating directory %s on remote: %v\n", remote_folder_name+path, err)
				return err
			}
		} else {
			b, err := os.ReadFile(directory_path + path)
			if err != nil {
				fmt.Printf("Error reading file %s\n", directory_path+path)
				return err
			}

			remote, err := sftpClient.OpenFile(remote_folder_name+path, os.O_CREATE|os.O_WRONLY|os.O_TRUNC)
			if err != nil {
				fmt.Printf("Error opening file %s on remote: %v\n", remote_folder_name+path, err)
				return err
			}

			_, err = remote.Write(b)
			if err != nil {
				fmt.Printf("Error writing to remote: %v\n", err)
				return err
			}
			err = remote.Close()
			if err != nil {
				fmt.Printf("Error closing remote file: %v\n", err)
				return err
			}
		}

		return nil
	})

	return err
}

func init() {
	postCmd.AddCommand(uploadCmd)

	uploadCmd.Flags().StringP("target", "t", ".", "Specifies the target directory to be uploaded. Default '.'")
	uploadCmd.Flags().StringP("server", "s", "", "Specifies the target bloggen server to upload to. If not specified, and no BLOGGEN_SERVER environment variable is configured, defaults to 'localhost:2222'")
	uploadCmd.Flags().StringP("keyfile", "k", "", "Specifies the target key file to use for ssh authentication. If not specified, defaults to `~/.ssh/id_rsa`")
	uploadCmd.Flags().Bool("no-conv", false, "Skips conversion of markdown files to HTML if they're already converted.")
}
