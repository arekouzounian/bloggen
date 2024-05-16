package cmd

import (
	"time"
)

type BlogPostMetaData struct {
	CreatedAt   time.Time
	LastChanged time.Time

	// author, title, tags, thumbnail/image?
}
