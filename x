#!/usr/bin/env bash
#
# x -- futhorc's task runner. Every task is a bash function holding exactly
# the commands you would type by hand; main() dispatches through a case
# allow-list so typos fail loudly; help derives the listing from the
# `## summary` lines.

set -euo pipefail

# Run from the repo root regardless of invocation dir.
cd "$(dirname "$0")"

build() { ## Compile the futhorc CLI to bin/futhorc.
	go build -o bin/futhorc ./cmd/futhorc
}

test() { ## Run the unit tests with the race detector.
	go test -race ./...
}

render() { ## Render a site: ./x render [futhorc flags] [site-dir].
	go run ./cmd/futhorc "$@"
}

help() { ## List the available tasks.
	echo "Usage: ./x <task>"
	echo
	echo "Tasks:"
	grep -E '^[a-z-]+\(\) \{ ##' "$0" \
		| sed -E 's/^([a-z-]+)\(\) \{ ## (.*)$/\1 \2/' \
		| sort \
		| awk '{ name = $1; $1 = ""; printf "  %-18s%s\n", name, substr($0, 2) }'
}

main() {
	local task="${1:-help}"
	case "$task" in
	build | test | render | help)
		"$task" "${@:2}"
		;;
	*)
		echo "x: unknown task '$task'" >&2
		echo >&2
		help >&2
		exit 1
		;;
	esac
}

main "$@"
