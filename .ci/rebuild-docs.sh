#! /bin/bash

cd "$(dirname "$(readlink -f "$0")")/.."

get_version() {
	GIT_DESC="$(git describe --all)"
	case "$GIT_DESC" in
		heads/master)
			echo -n latest
			;;
		heads/v*) 
			echo -n "${GIT_DESC#heads/v}"
		;;
	*)
		exit 1
	esac
}


set -x

cargo doc --no-deps --all

pwd

REPO_DIR=$(mktemp -d /tmp/XXXXXXX.pages)

git clone git@github.com:golemfactory/golem-client.git -b gh-pages "$REPO_DIR"

DOCS_VERDIR="$REPO_DIR/$(get_version)"

rsync -avr target/doc/ "$DOCS_VERDIR"

cd "$DOCS_VERDIR"

git add .
git commit -a --amend
pwd

git push -f

