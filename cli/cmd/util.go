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
// directory_path should always be an absolute path
func ValidateDirectoryStructure(directory_path string) (string, error) {
	if !filepath.IsAbs(directory_path) {
		return "", fmt.Errorf("provided directory path was not absolute: %s", directory_path)
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

// checks if any files matching the given extension exist in a directory
// `extension` must omit the '.' character.
//
// For example, to check if the file has the .html extension, you would call this function with `extension` as "html".
func FileExtensionExists(directory_path string, extension string) (bool, error) {
	files, err := os.ReadDir(directory_path)
	if err != nil {
		return false, err
	}

	for _, file := range files {
		filename := file.Name()
		ext_ind := strings.LastIndex(filename, ".")
		if ext_ind <= 0 {
			continue
		}

		if filename[ext_ind+1:] == extension {
			return true, nil
		}
	}
	return false, nil
}
