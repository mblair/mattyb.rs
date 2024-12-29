#!/usr/bin/env bash

set -ueo pipefail

freshen() {
	go install mvdan.cc/sh/cmd/shfmt@latest
	go install github.com/shurcooL/markdownfmt@latest
}

if [[ "$#" -gt 0 ]]; then
	while [[ "$#" -gt 0 ]]; do
		case "$1" in
		--freshen)
			freshen
			shift
			;;
		esac
	done
fi
