/*
Copyright © 2024 Arek Ouzounian <arek@arekouzounian.com>
*/
package util

import (
	"os"
	"path/filepath"

	"github.com/gomarkdown/markdown"
	"github.com/gomarkdown/markdown/ast"
	"github.com/gomarkdown/markdown/html"
	"github.com/gomarkdown/markdown/parser"
)

func GetDocumentAST(md []byte) ast.Node {
	extensions := parser.CommonExtensions | parser.AutoHeadingIDs | parser.NoEmptyLineBeforeBlock
	return parser.NewWithExtensions(extensions).Parse(md)
}

func RenderHTML(root ast.Node) []byte {
	htmlFlags := html.CommonFlags | html.HrefTargetBlank
	opts := html.RendererOptions{Flags: htmlFlags}
	renderer := html.NewRenderer(opts)

	return markdown.Render(root, renderer)
}

// validatedDirectoryPath is the directory with canonical structure
// that holds our files
func InterceptLinks(root ast.Node, validatedDirectoryPath string) (ast.Node, error) {
	var err error
	if !filepath.IsAbs(validatedDirectoryPath) {
		validatedDirectoryPath, err = filepath.Abs(validatedDirectoryPath)
		if err != nil {
			return nil, err
		}
	}
	target_folder := filepath.Join(validatedDirectoryPath, "assets")
	const FINAL_ENCODING = "/assets"

	ast.WalkFunc(root, func(node ast.Node, entering bool) ast.WalkStatus {
		link, is_link := node.(*ast.Link)
		img, is_img := node.(*ast.Image)

		if entering && (is_link || is_img) {
			var link_s string
			if is_link {
				link_s = string(link.Destination)
			} else {
				link_s = string(img.Destination)
			}

			stat, err := os.Stat(link_s)
			if err == nil {
				// real file, copy to local and convert
				source_abs, err := filepath.Abs(link_s)
				if err != nil {
					return ast.GoToNext
				}

				dest_abs := filepath.Join(target_folder, stat.Name())

				err = CopyFile(source_abs, dest_abs)
				if err != nil {
					return ast.Terminate
				}

				if is_link {
					link.Destination = []byte(filepath.Join(FINAL_ENCODING, stat.Name()))
				} else {
					img.Destination = []byte(filepath.Join(FINAL_ENCODING, stat.Name()))
				}
			}
		}

		return ast.GoToNext
	})

	if err != nil {
		return nil, err
	}
	return root, nil
}

// https://github.com/gomarkdown/markdown
func MDToHTML(md []byte) []byte {
	return RenderHTML(GetDocumentAST(md))
}
