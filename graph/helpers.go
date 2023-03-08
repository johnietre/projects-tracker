package graph

import (
  "errors"
  
  "github.com/johnietre/projects-tracker/database"
)

var serverErr = errors.New("Internal server error")

func NewConfig(dbPath string) (Config, error) {
  db, err := database.NewDB(dbPath)
  if err != nil {
    return Config{}, err
  }
  return Config{
    Resolvers: &Resolver{
      db: db,
    },
  }, nil
}
