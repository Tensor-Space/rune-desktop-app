.PHONY: download-models
download-models:
	@echo "Downloading models..."
	aws s3 sync s3://rune-models/ src-tauri/models/ --no-progress
