//go:generate go run github.com/99designs/gqlgen generate

package graph

import (
	"github.com/johnietre/projects-tracker/database"
)

// This file will not be regenerated automatically.
//
// It serves as dependency injection for your app, add any dependencies you require here.

type Resolver struct {
	db *database.DB
}

func (r *Resolver) CloseDB() error {
	return r.db.Close()
}
