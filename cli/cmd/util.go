package cmd

/*
Copyright © 2024 Arek Ouzounian <arek@arekouzounian.com>
*/

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/gomarkdown/markdown"
	"github.com/gomarkdown/markdown/html"
	"github.com/gomarkdown/markdown/parser"
)

// https://github.com/gomarkdown/markdown
// this is literally pulled directly from the readme

// TODO: scan for paths then pull into the asset folder
func mdToHTML(md []byte) []byte {
	extensions := parser.CommonExtensions | parser.AutoHeadingIDs | parser.NoEmptyLineBeforeBlock
	p := parser.NewWithExtensions(extensions)
	doc := p.Parse(md)

	// debug
	// children := doc.GetChildren()
	// for _, child := range children {
	// 	fmt.Println(child.AsContainer().Literal)
	// }

	// create HTML renderer with extensions
	htmlFlags := html.CommonFlags | html.HrefTargetBlank
	opts := html.RendererOptions{Flags: htmlFlags}
	renderer := html.NewRenderer(opts)

	return markdown.Render(doc, renderer)
}

// Returns the absolute path to the markdown file containing the post on success, or an error in the event of a failure.
func ValidateDirectoryStructure(directory_path string) (string, error) {
	if strings.LastIndex(directory_path, "/") != len(directory_path)-1 {
		directory_path += "/"
	}

	ls, err := os.ReadDir(directory_path)
	if err != nil {
		return "", err
	}

	requiredDirs := NewCheckList([]string{"assets"})
	requiredFiles := NewCheckList([]string{"meta.json"})
	foundFiles := make(map[string]bool, len(ls))

	for _, file := range ls {
		name := file.Name()

		fmt.Printf("Found file %s\n", name)

		if file.IsDir() {
			requiredDirs.TryCheck(name)
		} else {
			foundFiles[name] = true
			requiredFiles.TryCheck(name)
		}
	}

	// check for required files and required dirs
	if !requiredFiles.Satisfied() || !requiredDirs.Satisfied() {
		return "", fmt.Errorf(
			"missing required directories: %v. Missing required files: %v",
			requiredDirs.GetMissingItems(), requiredFiles.GetMissingItems(),
		)
	}

	// check for markdown file
	var mdFile *string

	for file := range foundFiles {
		ind := strings.LastIndex(file, ".")
		if ind >= 0 && file[ind:] == ".md" {
			mdFile = &file
			break
		}
	}
	if mdFile == nil {
		return "", fmt.Errorf("missing required markdown file")
	}

	// return full pathspec to md file

	path, err := filepath.Abs(directory_path + *mdFile)
	if err != nil {
		return "", err
	}

	return path, nil
}
