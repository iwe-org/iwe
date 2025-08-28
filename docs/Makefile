# Makefile for generating DOT exports and SVG visualizations
#
# This Makefile generates graph visualizations from the IWE documentation.
# It creates both basic and detailed exports, then converts them to SVG format using neato.

# Path to the IWE binary - adjust if needed
IWE_BIN = ../target/debug/iwe

# Output files
BASIC_DOT = docs-basic.dot
BASIC_SVG = docs-basic.svg
DETAILED_DOT = docs-detailed.dot
DETAILED_SVG = docs-detailed.svg

# Default target - generate all files
all:
	@echo "Generating basic DOT export with maximum depth..."
	$(IWE_BIN) export dot --depth 10 > $(BASIC_DOT)
	@echo "Converting basic DOT to SVG..."
	neato -Tsvg $(BASIC_DOT) -o $(BASIC_SVG)
	@echo "Generating detailed DOT export focused on index document..."
	$(IWE_BIN) export dot --key index --include-headers --depth 3 > $(DETAILED_DOT)
	@echo "Converting detailed DOT to SVG..."
	neato -Tsvg $(DETAILED_DOT) -o $(DETAILED_SVG)
	@echo "✓ All visualizations generated successfully"
	@echo "  - $(BASIC_SVG) (basic document relationships)"
	@echo "  - $(DETAILED_SVG) (index document with sections)"

# Declare phony targets
.PHONY: all

# Make verbose output conditional
ifndef VERBOSE
.SILENT:
endif
