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

type CheckList[K comparable] struct {
	RequiredItems map[K]bool
	required_left int
}

// If `item` is on the list and not yet checked off, it will check it off. Otherwise, it's ignored.
func (c *CheckList[K]) TryCheck(item K) {
	if sat, exists := c.RequiredItems[item]; exists {
		if !sat {
			c.required_left -= 1
		}
		c.RequiredItems[item] = true
	}
}

// Returns true if all items oon the RequiredItems list have been checked off.
func (c *CheckList[K]) Satisfied() bool {
	return c.required_left <= 0
}

func (c *CheckList[K]) GetMissingItems() []K {
	missing := make([]K, c.required_left)
	ind := 0

	for key, satisfied := range c.RequiredItems {
		if !satisfied {
			missing[ind] = key
			ind += 1
		}
	}

	return missing
}

// Returns a new checklist from the list of items.
func NewCheckList[K comparable](required_items []K) CheckList[K] {
	ret := CheckList[K]{
		RequiredItems: make(map[K]bool),
		required_left: len(required_items),
	}

	for _, x := range required_items {
		ret.RequiredItems[x] = false
	}

	return ret
}
