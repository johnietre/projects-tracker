package main

import (
	"flag"
	"log"
	"net/http"
  "os"

	"github.com/99designs/gqlgen/graphql/handler"
	"github.com/99designs/gqlgen/graphql/playground"
	"github.com/johnietre/projects-tracker/auth"
	"github.com/johnietre/projects-tracker/graph"
)

func main() {
	log.SetFlags(0)

	addr := flag.String("addr", "localhost:8000", "Address to run on")
	dbPath := flag.String("db", "", "Path to database")
  logPath := flag.String("log", "", "Path to log file")
	flag.Parse()

	if *dbPath == "" {
		log.Fatal("must provide database path")
	}

  if *logPath != "" {
    f, err := os.OpenFile(*logPath, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0644)
    if err != nil {
      log.Fatalln("error opening log file:", err)
    }
    log.SetOutput(f)
  }

	config, closeFunc, err := graph.NewConfig(*dbPath)
	if err != nil {
		log.Fatal(err)
	}
	defer closeFunc()
	srv := handler.NewDefaultServer(graph.NewExecutableSchema(config))

	http.Handle("/", http.FileServer(http.Dir(".")))
	http.Handle("/playground", playground.Handler("GraphQL playground", "/query"))
	http.Handle("/query", auth.Middleware(srv))

	//log.Printf("connect to http://%s/playground for GraphQL playground", *addr)
	log.Println("Running on", *addr)
	log.Fatal(http.ListenAndServe(*addr, nil))
}
