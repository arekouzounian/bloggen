/*
Copyright © 2024 Arek Ouzounian <arek@arekouzounian.com>
*/
package util

import (
	"bufio"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"
)

// Returns the absolute path to the markdown file containing the post on success, or an error in the event of a failure.
// directory_path should always be an absolute path
func ValidateDirectoryStructure(directory_path string) (*ValidateDirectoryStructureResult, error) {
	if !filepath.IsAbs(directory_path) {
		var err error
		directory_path, err = filepath.Abs(directory_path)
		if err != nil {
			return nil, fmt.Errorf("unable to convert to absolute path: %v", err)
		}
	}

	ls, err := os.ReadDir(directory_path)
	if err != nil {
		return nil, err
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
		return nil, fmt.Errorf(
			"missing required directories: %v. Missing required files: %v",
			requiredDirs.GetMissingItems(), requiredFiles.GetMissingItems(),
		)
	}

	// check for markdown file
	var mdFile *string
	for file := range foundFiles {
		if filepath.Ext(file) == ".md" {
			mdFile = &file
			break
		}
	}
	if mdFile == nil {
		return nil, fmt.Errorf("missing required markdown file")
	}

	// return full pathspec to md file
	path := filepath.Join(directory_path, *mdFile)
	assets := filepath.Join(directory_path, "assets")
	meta := filepath.Join(directory_path, "meta.json")

	res := &ValidateDirectoryStructureResult{
		MarkdownFilePath: path,
		AssetsDirPath:    assets,
		MetaFilePath:     meta,
	}

	return res, nil
}

// checks if any files matching the given extension exist in a directory
// `extension` mustn't omit the '.' character.
//
// For example, to check if the file has the .html extension, you would call this function with `extension` as ".html".
func FileExtensionExists(directory_path string, extension string) (bool, error) {
	files, err := os.ReadDir(directory_path)
	if err != nil {
		return false, err
	}

	for _, file := range files {
		filename := file.Name()
		if filepath.Ext(filename) == extension {
			return true, nil
		}
	}
	return false, nil
}

// copies files 1kb at a time to keep low mem footprint
// Should always take in absolute paths
func CopyFile(source_path string, dest_path string) error {
	buf := make([]byte, 1024)

	source, err := os.Open(source_path)
	if err != nil {
		return err
	}
	defer source.Close()

	dest, err := os.OpenFile(dest_path, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, 0666)
	if err != nil {
		return err
	}
	defer dest.Close()

	bytes_read, err := source.Read(buf)
	for bytes_read != 0 && err == nil {
		_, err = dest.Write(buf[:bytes_read])
		if err != nil {
			return err
		}

		bytes_read, err = source.Read(buf)
	}

	return nil
}

// `dir_path` is the path to the directory where `meta.json` will be housed
func WriteMetaDataFromInput(dir_path string) error {
	var Author string
	var Title string
	var Description string

	scanner := bufio.NewScanner(os.Stdin)

	fmt.Println("Enter Post Metadata, or hit Enter for default: ")
	for {
		fmt.Print("\tPost Author (default ''): ")
		scanner.Scan()
		if scanner.Err() != nil {
			fmt.Println("Invalid argument. Please try again.")
			continue
		}

		Author = scanner.Text()
		break
	}

	for {
		fmt.Print("\tPost Title (defaults to name of parent folder): ")
		scanner.Scan()
		if scanner.Err() != nil {
			fmt.Println("Invalid Argument. Please try again.")
			continue
		}

		txt := scanner.Text()
		if txt == "" {
			txt = filepath.Base(dir_path)
		}

		Title = txt
		break
	}

	for {
		fmt.Print("\tPost Description (default ''): ")
		scanner.Scan()
		if err := scanner.Err(); err != nil {
			fmt.Println("Invalid Argument. Please try again.")
			fmt.Println(err.Error())
			continue
		}

		Description = scanner.Text()
		break
	}

	ser := BlogPostMetaData{
		LastChanged: time.Now().Unix(),
		Author:      Author,
		Title:       Title,
		Description: Description,
	}

	b, err := json.Marshal(ser)
	if err != nil {
		return err
	}

	return os.WriteFile(filepath.Join(dir_path, "meta.json"), b, 0666)
}

func UpdateTimeStampsInMeta(meta_file_path string, timestamp int64) error {
	b, err := os.ReadFile(meta_file_path)
	if err != nil {
		return err
	}

	var meta BlogPostMetaData

	err = json.Unmarshal(b, &meta)
	if err != nil {
		return err
	}

	meta.LastChanged = timestamp

	b, err = json.Marshal(meta)
	if err != nil {
		return err
	}

	return os.WriteFile(meta_file_path, b, 0666)
}

// Checks if the post name for `bloggen post init` is valid
// Essentially just checks that it doesn't have whitespace or special characters
func IsValidPostName(post_name string) bool {
	post_name = strings.ToLower(post_name)

	for _, r := range post_name {
		if (r < 97 || r > 122) && r != 45 {
			return false
		}
	}
	return true
}
