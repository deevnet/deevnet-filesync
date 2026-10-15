.PHONY: help html server clean

default: help

help:
	@echo "Deevnet FileSync Docs"
	@echo ""
	@echo "Usage: make <target>"
	@echo ""
	@echo "Targets:"
	@echo "  html            Build the site to public/"
	@echo "  server          Run local development server with live reload"
	@echo "  clean           Remove build artifacts"

# Build the site to public/
html:
	hugo --minify

# Run local development server with live reload
server:
	hugo server --bind 0.0.0.0 --baseURL http://localhost:1313/deevnet-filesync/

# Clean build artifacts
clean:
	rm -rf public/ resources/ .hugo_build.lock
