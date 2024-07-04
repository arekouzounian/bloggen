/*
Copyright © 2024 Arek Ouzounian <arek@arekouzounian.com>
*/
package cmd

import (
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"strings"

	"github.com/arekouzounian/bloggen/util"
	"github.com/pkg/sftp"
	"github.com/spf13/cobra"
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

		target, err = filepath.Abs(target)
		if err != nil {
			fmt.Printf("Fatal: Not a valid directory: %s\n", target)
			return
		}

		res, err := util.ValidateDirectoryStructure(target)
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

		err = util.UpdateTimeStampsInMeta(res.MetaFilePath, stat.ModTime().Unix())
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
			ast, err := util.InterceptLinks(util.GetDocumentAST(file), filepath.Dir(md_file))
			if err != nil {
				fmt.Printf("Fatal: Error intercepting document links: %v\n", err)
				return
			}
			new_path := md_file[:strings.LastIndex(md_file, ".")] + ".html"

			err = os.WriteFile(new_path, util.RenderHTML(ast), 0666)
			if err != nil {
				fmt.Printf("Fatal: Error writing to file %s: %v\n", new_path, err)
				return
			}
		} else {
			// check that html file exists
			hasHtml, err := util.FileExtensionExists(target, ".html")
			if err != nil {
				fmt.Printf("Fatal: Unable to read target directory %s: %v", target, err)
				return
			}
			if !hasHtml {
				fmt.Printf("Fatal: Specified target directory (%s) lacks an HTML file. Consider using `bloggen conv`, or removing the `no-conv` flag.\n", target)
				return
			}
		}

		err = UploadPost(target, Host, Keypath, Hostsfile)
		if err != nil {
			fmt.Println(err.Error())
			return
		}

		fmt.Println("Post uploaded to server successfully.")
	},
}

func UploadPost(directory_path string, host string, keypath string, hostsfile string) error {
	conn, err := util.NewSSHClient(host, keypath, hostsfile)
	if err != nil {
		return err
	}
	defer conn.Close()

	sftpClient, err := sftp.NewClient(conn)
	if err != nil {
		fmt.Printf("Failed to create sftp client: %v", err)
		return err
	}
	defer sftpClient.Close()

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
	uploadCmd.Flags().Bool("no-conv", false, "Skips conversion of markdown files to HTML if they're already converted.")
}
