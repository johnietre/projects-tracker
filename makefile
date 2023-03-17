.PHONY: generate server web

generate: graph/*.graphql
	go generate ./...

server:
	go build -o bin/server server/server.go

web:
	wasm-pack build --target web
