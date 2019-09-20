#!/usr/bin/env bash
set -eou pipefail

export HOME=/cache
export YARN_CACHE_FOLDER=/cache

cd app
yarn

trap 'kill %1' EXIT
yarn clean
yarn build
yarn server&

#yarn test:cypress:run
exit 1
