package cmd

/*
Copyright © 2024 Arek Ouzounian <arek@arekouzounian.com>
*/

import (
	"fmt"
	"os"
	"strings"

	"github.com/arekouzounian/bloggen/util"
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

		flag, err := cmd.Flags().GetString("output")
		if err != nil {
			panic(err)
		} else if flag == "" {
			fmt.Println("Output flag not specified, defaulting to current directory.")
		}

		err = ConvertMDToHTML(args[0], flag)
		if err != nil {
			fmt.Println(err.Error())
		}

	},
}

// Converts `file_path` to HTML and saves it in `target_location`.
func ConvertMDToHTML(file_path string, target_location string) error {
	f, err := os.Stat(file_path)
	if err != nil {
		return err
	}

	file, err := os.ReadFile(file_path)
	if err != nil {
		return err
	}

	conv := util.MDToHTML(file)

	var name string
	ext := ".html"
	if target_location == DEFAULT_OUTPUT {
		name = strings.Split(f.Name(), ".")[0]
	} else {
		name = target_location
		ext = ""
	}

	// err = os.WriteFile(name+mod+EXT, conv, fs.FileMode(os.O_RDWR))
	new_file, err := os.Create(name + ext)
	if err != nil {
		return err
	}
	defer new_file.Close()

	_, err = new_file.Write(conv)
	if err != nil {
		return err
	}

	fmt.Printf("File created at %s\n", name+ext)

	return nil
}

func init() {
	rootCmd.AddCommand(conv)
	conv.Flags().StringP("output", "o", "", "Output location or filename for the resultant conversion")
}
