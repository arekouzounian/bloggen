package cmd

/*
Copyright © 2024 Arek Ouzounian <arek@arekouzounian.com>
*/

import (
	"fmt"
	"os"
	"strings"

	"github.com/spf13/cobra"
)

const (
	DEFAULT_OUTPUT = ""
)

var conv = &cobra.Command{
	Use:   "conv",
	Short: "Converts markdown to HTML",
	Long: `Converts the given markdown document to HTML. 
An output name or location can be specified with the -o flag. 

Ex: 
	bloggen conv test.md -o ~/test-conv.html`,
	Run: func(cmd *cobra.Command, args []string) {
		if len(args) < 1 {
			fmt.Println("No file specified")
			fmt.Println("Use the --help or -h flag to see command usage")
			return
		}

		// MIME-type checking?
		f, err := os.Stat(args[0])
		if err != nil {
			fmt.Println(err.Error())
			return
		}

		file, err := os.ReadFile(args[0])
		if err != nil {
			fmt.Println(err.Error())
			return
		}

		conv := mdToHTML(file)
		// fmt.Println(string(conv))

		flag, err := cmd.Flags().GetString("output")
		if err != nil {
			panic(err)
		} else if flag == "" {
			fmt.Println("Output flag not specified, defaulting to current directory.")
		}

		var name string
		ext := ".html"
		if flag == DEFAULT_OUTPUT {
			name = strings.Split(f.Name(), ".")[0]
		} else {
			name = flag
			ext = ""
		}

		// err = os.WriteFile(name+mod+EXT, conv, fs.FileMode(os.O_RDWR))
		new_file, err := os.Create(name + ext)
		if err != nil {
			fmt.Println("Error creating file:", name+ext)
			fmt.Println(err.Error())
			return
		}
		defer new_file.Close()

		_, err = new_file.Write(conv)
		if err != nil {
			fmt.Println("Error writing to file:", err.Error())
		}

		fmt.Printf("File created at %s\n", name+ext)
	},
}

func init() {
	rootCmd.AddCommand(conv)
	conv.Flags().StringP("output", "o", "", "Output location or filename for the resultant conversion")
}
