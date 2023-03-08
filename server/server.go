package main

import (
	"log"
	"net/http"
	"os"

	"github.com/99designs/gqlgen/graphql/handler"
	"github.com/99designs/gqlgen/graphql/playground"
	"github.com/johnietre/projects-tracker/auth"
	"github.com/johnietre/projects-tracker/graph"
)

const defaultPort = "8080"

func main() {
	port := os.Getenv("PORT")
	if port == "" {
		port = defaultPort
	}

	//srv := handler.NewDefaultServer(graph.NewExecutableSchema(graph.Config{Resolvers: &graph.Resolver{}}))
  config, err := graph.NewConfig("proj-trak.db")
  if err != nil {
    log.Fatal(err)
  }
	srv := handler.NewDefaultServer(graph.NewExecutableSchema(config))

	http.Handle("/playground", playground.Handler("GraphQL playground", "/query"))
	http.Handle("/query", auth.Middleware(srv))

	log.Printf("connect to http://localhost:%s/playground for GraphQL playground", port)
	log.Fatal(http.ListenAndServe(":"+port, nil))
}
