#!/bin/bash
set -e

GIT_HASH=$(git rev-parse HEAD)
echo "using git hash $GIT_HASH"

echo "$DOCKER_PASSWORD" | docker login -u "$DOCKER_USERNAME" --password-stdin

cp -rf ../proto-tmp ./proto
docker pull sm64js/sm64js-build || true
docker build --cache-from=sm64js/sm64js-build -t sm64js/sm64js-build:latest .
docker push sm64js/sm64js-build:latest
docker tag sm64js/sm64js-build sm64js/sm64js-build:$GIT_HASH
docker push sm64js/sm64js-build:$GIT_HASH
rm -rf ./proto
