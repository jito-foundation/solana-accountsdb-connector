#!/usr/bin/env sh
# Builds this program in a docker container.
# Useful for running on machines that might not have cargo installed but can run docker (Flatcar Linux).
set -eux

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"

GIT_SHA="$(git describe --always --dirty)"

echo $GIT_SHA

DOCKER_BUILDKIT=1 docker build \
  --build-arg ci_commit=$GIT_SHA \
  -t jitolabs/solana-accountsdb-connector \
  -f Dockerfile . \
  --progress=plain

# Creates a temporary container, copies solana-accountsdb-connector built inside container there and
# removes the temporary container.
docker rm temp || true
docker container create --name temp jitolabs/solana-accountsdb-connector

# Outputs the binary to $SCRIPT_DIR/docker-output
mkdir -p $SCRIPT_DIR/docker-output
docker container cp temp:/solana-accountsdb-connector/docker-output $SCRIPT_DIR/
docker rm temp
