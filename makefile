.PHONY: generate server

generate: graph/*.graphql
	go generate ./...

server:
	go build -o bin/server server.go
