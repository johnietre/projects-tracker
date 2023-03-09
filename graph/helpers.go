package graph

import (
  "errors"
  
  "github.com/johnietre/projects-tracker/database"
)

type CloseFunc func() error

var serverErr = errors.New("Internal server error")

func NewConfig(dbPath string) (Config, CloseFunc, error) {
  db, err := database.NewDB(dbPath)
  if err != nil {
    return Config{}, nil, err
  }
  r := &Resolver{
    db: db,
  }
  return Config{Resolvers: r}, CloseFunc(r.CloseDB), nil
}
