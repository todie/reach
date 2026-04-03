#!/usr/bin/env bash
# Install Scrapling browser dependencies inside the container.
set -euo pipefail

echo "Installing Scrapling fetchers..."
scrapling install

echo "Scrapling setup complete."
