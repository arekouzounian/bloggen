package cmd

/*
Copyright © 2024 Arek Ouzounian <arek@arekouzounian.com>
*/

import (
	"time"
)

type BlogPostMetaData struct {
	CreatedAt   time.Time
	LastChanged time.Time

	// author, title, tags, thumbnail/image?
}
