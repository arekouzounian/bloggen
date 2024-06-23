/*
Copyright © 2024 Arek Ouzounian <arek@arekouzounian.com>
*/
package cmd

import (
	"fmt"
	"io/fs"
	"log"
	"net"
	"os"
	"path/filepath"
	"strings"

	"github.com/pkg/sftp"
	"github.com/spf13/cobra"
	"golang.org/x/crypto/ssh"

	"github.com/skeema/knownhosts"
)

const (
	DEFAULT_SERVER_ADDR = "localhost:2222"
	SERVER_ENV_KEY      = "BLOGGEN_SERVER"
	EMPTY_STR           = ""
)

// uploadCmd represents the upload command
var uploadCmd = &cobra.Command{
	Use:   "upload",
	Short: "Uploads the post from the target directory to the given server",
	Long: `Sends the given directory (default '.') to the bloggen server via sftp.
The directory should be in canonical bloggen structure; you can create the proper structure using the 'bloggen post init' command.

This command will automatically attempt to convert the contained markdown to HTML. If you want to stop this, you can use the --no-conv flag.

When converting to HTML, links to files within the markdown document will automatically be intercepted, and these files will be copied into the assets folder.
To this end, make sure that all of your linked local files are accessible, and have absolute paths.`,
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
		hostsfile, err := cmd.Flags().GetString("hostfile")
		if err != nil {
			fmt.Println("Fatal: hostfile flag not found")
			return
		}

		if host == EMPTY_STR {
			if env_key, exists := os.LookupEnv(SERVER_ENV_KEY); exists {
				host = env_key
			} else {
				host = DEFAULT_SERVER_ADDR
			}
		}

		homedir, err := os.UserHomeDir()
		if err != nil {
			fmt.Printf("Error accessing default keyfile: %v", err)
			return
		}
		if keypath == EMPTY_STR {
			keypath = filepath.Join(homedir, ".ssh", "id_rsa")
		}
		if hostsfile == EMPTY_STR {
			hostsfile = filepath.Join(homedir, ".ssh", "known_hosts")
		}

		target, err = filepath.Abs(target)
		if err != nil {
			fmt.Printf("Fatal: Not a valid directory: %s\n", target)
			return
		}

		res, err := ValidateDirectoryStructure(target)
		if err != nil {
			fmt.Printf("Fatal: Invalid directory structure: %v\n", err)
			return
		}

		md_file := res.MarkdownFilePath
		stat, err := os.Stat(md_file)
		if err != nil {
			fmt.Printf("Fatal: unable to stat markdown file %s: %v\n", md_file, err)
			return
		}

		err = UpdateTimeStampsInMeta(res.MetaFilePath, stat.ModTime().Unix())
		if err != nil {
			fmt.Printf("Fatal: Unable to update timestamps in meta.json: %v\n", err)
			return
		}

		if !no_conv {
			file, err := os.ReadFile(md_file)
			if err != nil {
				fmt.Printf("Fatal: Unable to read file %s: %v\n", md_file, err)
				return
			}
			ast, err := InterceptLinks(GetDocumentAST(file), filepath.Dir(md_file))
			if err != nil {
				fmt.Printf("Fatal: Error intercepting document links: %v\n", err)
				return
			}
			new_path := md_file[:strings.LastIndex(md_file, ".")] + ".html"

			err = os.WriteFile(new_path, RenderHTML(ast), 0666)
			if err != nil {
				fmt.Printf("Fatal: Error writing to file %s: %v\n", new_path, err)
				return
			}
		} else {
			// check that html file exists
			hasHtml, err := FileExtensionExists(target, ".html")
			if err != nil {
				fmt.Printf("Fatal: Unable to read target directory %s: %v", target, err)
				return
			}
			if !hasHtml {
				fmt.Printf("Fatal: Specified target directory (%s) lacks an HTML file. Consider using `bloggen conv`, or removing the `no-conv` flag.\n", target)
				return
			}
		}

		err = UploadPost(target, host, keypath, hostsfile)
		if err != nil {
			fmt.Println(err.Error())
			return
		}

		fmt.Println("Post uploaded to server successfully.")
	},
}

func UploadPost(directory_path string, host string, keypath string, hostsfile string) error {
	key, err := os.ReadFile(keypath)
	if err != nil {
		return fmt.Errorf("unable to read private key: %v", err)
	}

	// Create the Signer for this private key.
	signer, err := ssh.ParsePrivateKey(key)
	if err != nil {
		return fmt.Errorf("unable to parse private key: %v", err)
	}

	kh, err := knownhosts.New(hostsfile)
	if err != nil {
		return fmt.Errorf("Error parsing hostfile %s: %v", hostsfile, err)
	}

	// Create a custom permissive hostkey callback which still errors on hosts
	// with changed keys, but allows unknown hosts and adds them to known_hosts
	cb := ssh.HostKeyCallback(func(hostname string, remote net.Addr, key ssh.PublicKey) error {
		err := kh(hostname, remote, key)
		if knownhosts.IsHostKeyChanged(err) {
			return fmt.Errorf("REMOTE HOST IDENTIFICATION HAS CHANGED for host %s! This may indicate a MitM attack.", hostname)
		} else if knownhosts.IsHostUnknown(err) {
			f, ferr := os.OpenFile(hostsfile, os.O_APPEND|os.O_WRONLY, 0600)
			if ferr == nil {
				defer f.Close()
				ferr = knownhosts.WriteKnownHost(f, hostname, remote, key)
			}
			if ferr == nil {
				log.Printf("Added host %s to known_hosts\n", hostname)
			} else {
				log.Printf("Failed to add host %s to known_hosts: %v\n", hostname, ferr)
			}
			return nil // permit previously-unknown hosts (warning: may be insecure)
		}
		return err
	})

	config := &ssh.ClientConfig{
		Auth: []ssh.AuthMethod{
			ssh.PublicKeys(signer),
		},
		HostKeyCallback:   cb,
		HostKeyAlgorithms: kh.HostKeyAlgorithms(host),
	}

	// Connect to SSH server
	conn, err := ssh.Dial("tcp", host, config)
	if err != nil {
		return fmt.Errorf("failed to connect to SSH server: %v", err)
	}
	defer conn.Close()

	sftpClient, err := sftp.NewClient(conn)
	if err != nil {
		fmt.Printf("Failed to create sftp client: %v", err)
		return err
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

		remote_path := filepath.Join(remote_folder_name, path)
		if entry.IsDir() {
			err = sftpClient.Mkdir(remote_path)
			if err != nil {
				fmt.Printf("Error creating directory %s on remote: %v\n", remote_path, err)
				return err
			}
		} else {
			b, err := os.ReadFile(filepath.Join(directory_path, path))
			if err != nil {
				fmt.Printf("Error reading file %s\n", directory_path+path)
				return err
			}

			remote, err := sftpClient.OpenFile(remote_path, os.O_CREATE|os.O_WRONLY|os.O_TRUNC)
			if err != nil {
				fmt.Printf("Error opening file %s on remote: %v\n", remote_path, err)
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
	uploadCmd.Flags().StringP("hostfile", "f", "", "Specifies the target known_hosts file to use for ssh authentication. If not specified, defaults to `~/.ssh/known_hosts`")
	uploadCmd.Flags().Bool("no-conv", false, "Skips conversion of markdown files to HTML if they're already converted.")
}
