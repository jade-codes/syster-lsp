.PHONY: help build run test clean fmt lint check run-guidelines package

# Default values for cross-platform builds
TARGET ?= 
ARTIFACT ?= syster-lsp

help:
	@echo "Available targets:"
	@echo "  build              - Build the project"
	@echo "  release            - Build release binary"
	@echo "  run                - Run the project"
	@echo "  test               - Run tests"
	@echo "  clean              - Clean build artifacts"
	@echo "  fmt                - Format code with rustfmt"
	@echo "  lint               - Run clippy linter"
	@echo "  check              - Run fmt + lint + test"
	@echo "  run-guidelines     - Run complete validation (fmt + lint + build + test)"
	@echo "  package            - Build release package for distribution"
	@echo "  install-vscode-lsp - Copy release binary to vscode-lsp for local dev"

build:
	cargo build

release:
	cargo build --release

run:
	cargo run

test:
	cargo test

test-verbose:
	cargo test -- --nocapture

clean:
	cargo clean

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

lint:
	cargo clippy --all-targets -- -D warnings

check: fmt-check lint test

run-guidelines:
	@echo "=== Running Complete Validation Pipeline ==="
	@echo ""
	@echo "Step 1/3: Formatting code..."
	@cargo fmt
	@echo "✓ Code formatted"
	@echo ""
	@echo "Step 2/3: Running linter (includes build)..."
	@cargo clippy --all-targets -- -D warnings
	@echo "✓ Linting passed"
	@echo ""
	@echo "Step 3/3: Running tests (limited parallelism for CI)..."
	@cargo test -- --test-threads=4
	@echo ""
	@echo "=== ✓ All guidelines passed! ==="

# Build release package with optional cross-compilation target
# Usage: make package TARGET=x86_64-pc-windows-msvc ARTIFACT=syster-windows-x64
package:
	@echo "Building package..."
ifdef TARGET
	@echo "Cross-compiling for $(TARGET)..."
	cargo build --release --target $(TARGET)
	@mkdir -p dist
	@# Copy binary to dist
	@if echo "$(TARGET)" | grep -q "windows"; then \
		cp target/$(TARGET)/release/syster-lsp.exe dist/syster-lsp.exe; \
	else \
		cp target/$(TARGET)/release/syster-lsp dist/syster-lsp; \
		chmod +x dist/syster-lsp; \
	fi
	@# Copy sysml.library to dist
	cp -r crates/syster-lsp/sysml.library dist/sysml.library
	@# Create archive
	@if echo "$(TARGET)" | grep -q "windows"; then \
		cd dist && powershell -command "Compress-Archive -Force -Path syster-lsp.exe,sysml.library -DestinationPath ../$(ARTIFACT).zip" && cd ..; \
	else \
		cd dist && tar -czvf ../$(ARTIFACT).tar.gz syster-lsp sysml.library && cd ..; \
	fi
	@rm -rf dist
	@echo "✓ Package created: $(ARTIFACT).tar.gz or $(ARTIFACT).zip"
else
	@cargo build --release
	@echo "✓ Package built (use TARGET= and ARTIFACT= for release packaging)"
endif

# Copy release binary to vscode-lsp server directory for local development
# Usage: make install-vscode-lsp VSCODE_LSP_DIR=../syster/editors/vscode-lsp
VSCODE_LSP_DIR ?= ../syster/editors/vscode-lsp
install-vscode-lsp: release
	@echo "Installing to $(VSCODE_LSP_DIR)/server..."
	@mkdir -p $(VSCODE_LSP_DIR)/server
	@# Determine platform-specific binary name
	@UNAME=$$(uname -s); ARCH=$$(uname -m); \
	if [ "$$UNAME" = "Linux" ]; then \
		BINARY_NAME="syster-lsp-linux-x64"; \
	elif [ "$$UNAME" = "Darwin" ]; then \
		if [ "$$ARCH" = "arm64" ]; then \
			BINARY_NAME="syster-lsp-darwin-arm64"; \
		else \
			BINARY_NAME="syster-lsp-darwin-x64"; \
		fi; \
	else \
		BINARY_NAME="syster-lsp-win32-x64.exe"; \
	fi; \
	cp target/release/syster-lsp $(VSCODE_LSP_DIR)/server/$$BINARY_NAME; \
	chmod +x $(VSCODE_LSP_DIR)/server/$$BINARY_NAME; \
	echo "✓ Copied to $(VSCODE_LSP_DIR)/server/$$BINARY_NAME"
	@# Copy sysml.library if not already present
	@if [ ! -d "$(VSCODE_LSP_DIR)/server/sysml.library" ]; then \
		cp -r crates/syster-lsp/sysml.library $(VSCODE_LSP_DIR)/server/sysml.library; \
		echo "✓ Copied sysml.library"; \
	fi
