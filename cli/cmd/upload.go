/*
Copyright © 2024 Arek Ouzounian <arek@arekouzounian.com>
*/
package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
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

		_, err = cmd.Flags().GetBool("no-conv")
		if err != nil {
			fmt.Println("Fatal: no-conv flag not found")
			return
		}

		path, err := ValidateDirectoryStructure(target)
		if err != nil {
			fmt.Printf("Invalid directory structure: %v\n", err)
			return
		}

		fmt.Println(path)

		// if !no_conv {
		// 	ind := strings.LastIndex(md_file, "/")
		// 	if ind >= 0 {
		// 		new_path := md_file[:strings.LastIndex(md_file, ".")] + ".html"
		// 		ConvertMDToHTML(md_file, new_path, false)
		// 	} else {
		// 		fmt.Printf("Path to blog post is not absolute: %s\n", md_file)
		// 		return
		// 	}
		// }

		// UploadPost(&target)
	},
}

func UploadPost(directory_path *string) {
	fmt.Println(directory_path)
}

func init() {
	postCmd.AddCommand(uploadCmd)

	uploadCmd.Flags().StringP("target", "t", ".", "Specifies the target directory to be uploaded. Default '.'")
	uploadCmd.Flags().Bool("no-conv", false, "Skips conversion of markdown files to HTML if they're already converted.")
}
