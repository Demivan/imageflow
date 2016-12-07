#!/bin/bash
set -e

#export EXTRA_DOCKER_BUILD_PARAMS=--no-cache
#export DISABLE_COMPILATION_CACHES=True

docker_images=(build_if_gcc54 build_if_gcc48)
for DOCKER_IMAGE in "${docker_images[@]}"
do
  ./test.sh ${DOCKER_IMAGE}
done

