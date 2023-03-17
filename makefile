.PHONY: all generate server web

generate: graph/*.graphql
	go generate ./...

all: server web

server:
	go build -o bin/server server/server.go

web:
	wasm-pack build --target web
