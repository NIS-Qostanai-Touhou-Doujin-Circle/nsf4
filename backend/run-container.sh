#!/bin/sh
# Run the NSF backend container with the appropriate environment variables
# Usage: ./run-container.sh [container-name]

CONTAINER_NAME=${1:-nsf-backend}

docker run -d \
  --name $CONTAINER_NAME \
  --restart unless-stopped \
  -p 5123:5123 \
  -e DATABASE_URL="mysql://user:password@db-host:3306/nsf" \
  -e PORT="5123" \
  -e MEDIA_SERVER_URL="rtmp://media-server:1935" \
  -e SCREENSHOT_INTERVAL_SECONDS="10" \
  -e SCREENSHOT_QUALITY="80" \
  -e RUST_LOG="info" \
  nsf-backend:latest

echo "Container $CONTAINER_NAME started on port 5123"
