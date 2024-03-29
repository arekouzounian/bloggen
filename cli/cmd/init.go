/*
Copyright © 2023 NAME HERE <EMAIL ADDRESS>
*/
package cmd

import (
	"encoding/json"
	"fmt"
	"os"
	"time"

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

		meta := BlogPostMetaData{
			CreatedAt:   time.Now(),
			LastChanged: time.Now(),
		}
		json, err := json.Marshal(meta)
		if err != nil {
			fmt.Println("Fatal: ", err.Error())
			return
		}
		err = os.WriteFile(output+"meta.json", json, os.FileMode(0666))
		if err != nil {
			fmt.Println("Error creating meta.json")
			fmt.Println(err.Error())
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

	// Here you will define your flags and configuration settings.

	// Cobra supports Persistent Flags which will work for this command
	// and all subcommands, e.g.:
	// initCmd.PersistentFlags().String("foo", "", "A help for foo")

	// Cobra supports local flags which will only run when this command
	// is called directly, e.g.:
	// initCmd.Flags().BoolP("toggle", "t", false, "Help message for toggle")
	initCmd.Flags().StringP("output", "o", ".", "specifies the directory that the new subdirectory will be located within.")
}
