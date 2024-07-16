/*
Copyright © 2024 Arek Ouzounian <arek@arekouzounian.com>
*/
package cmd

import (
	"fmt"
	"os"

	"github.com/arekouzounian/bloggen/util"
	"github.com/spf13/cobra"
)

// initCmd represents the init command
var initCmd = &cobra.Command{
	Use:   "init [blog post name]",
	Short: "Creates a new blog post directory structure.",
	Long: `This will create a directory (whose location can be specified with the -o or --output flag) whose structure resembles the following:
	[blog post name]/
	  |_ meta.json
	  |_ [blog post name].md
	  |_assets/
	    |...

The assets folder is where you can place anything you may be referencing in the main markdown document, such as images or files. The meta.json file will contain important metadata about the blog post.`,
	Run: func(cmd *cobra.Command, args []string) {
		if len(args) < 1 {
			fmt.Println("A blog post name argument is needed.")
			return
		}
		name := args[0]
		if !util.IsValidPostName(name) {
			fmt.Println("Blog post names must be alphanumeric, separated by '-' characters if desired. You will be able to have more specialized post titles under the 'Post Title' prompt.")
			return
		}

		output, err := cmd.Flags().GetString("output")
		if err != nil {
			fmt.Println(err.Error())
			return
		}

		if output == "." {
			output = name
		} else {
			if output[len(output)-1] != '/' {
				output += "/"
			}

			output += name
		}

		err = os.Mkdir(output, os.FileMode(0755))
		if err != nil {
			fmt.Println("Error creating directory", output)
			fmt.Println(err.Error())
			return
		}

		// directory exists at <output>/ now.
		output += "/"

		err = util.WriteMetaDataFromInput(output)
		if err != nil {
			fmt.Printf("Error creating meta.json: %v", err)
			return
		}

		template := `# Hello, World!
---
This is a file template. Feel free to get rid of this and replace it with your own content!`
		err = os.WriteFile(output+name+".md", []byte(template), os.FileMode(0666))
		if err != nil {
			fmt.Println("Error creating", name+".md")
			fmt.Println(err.Error())
			return
		}

		err = os.Mkdir(output+"assets", os.FileMode(0755))
		if err != nil {
			fmt.Println("Error creating directory", output+"assets")
			fmt.Println(err.Error())
			return
		}
		fmt.Println("Initialization succeeded. Directory structure can be located at " + output)
	},
}

func init() {
	postCmd.AddCommand(initCmd)

	initCmd.Flags().StringP("output", "o", ".", "specifies the directory that the new subdirectory will be located within.")
}
