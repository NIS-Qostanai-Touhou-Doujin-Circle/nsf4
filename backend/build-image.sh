#!/bin/sh
# Build the Docker image for the NSF backend
# Usage: ./build-image.sh [tag]

TAG=${1:-latest}

echo "Building nsf-backend:$TAG..."
docker build -t nsf-backend:$TAG -f DOCKERFILE .

echo "Build complete!"
echo "To run the container: docker run -p 5123:5123 nsf-backend:$TAG"
echo "Or use the run-container.sh script for a more comprehensive setup."
