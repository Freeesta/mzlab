#!/bin/sh
# Installs mzlab-cli from a GitHub release of Freeesta/mzlab: picks the binary of this OS and CPU, checks it
# against the release's SHA256SUMS and copies it into a user-writable folder (no sudo). No telemetry: the only
# requests are the binary and SHA256SUMS. POSIX sh; tested by tests/test_install_sh.py against a local server.
#   curl -fsSL https://raw.githubusercontent.com/Freeesta/mzlab/main/tools/install.sh | sh
#   sh install.sh [--version X.Y.Z] [--dir DIR] [--dry-run]
# Exit codes: 0 installed, 2 wrong usage, 3 unsupported platform, 4 no curl/wget or no SHA-256 tool,
# 5 the release has no asset for this platform, 6 download failed, 7 checksum missing or different, 8 install failed.

set -eu

REPO="Freeesta/mzlab"
FALLBACK="build from source instead: git clone https://github.com/$REPO && cd mzlab && cargo build --profile cli -p mzlab-cli (binary in target/cli/mzlab-cli)"

say() { printf 'mzlab install: %s\n' "$*" >&2; }
die() { code=$1; shift; printf 'mzlab install: error: %s\n' "$*" >&2; exit "$code"; }

usage() {
	cat <<EOF
Usage: install.sh [--version X.Y.Z] [--dir DIR] [--dry-run]
  --version X.Y.Z  install that release (default: the latest published release; env MZLAB_VERSION)
  --dir DIR        install folder (default: \$HOME/.local/bin; env MZLAB_INSTALL_DIR)
  --dry-run        print the platform, URLs and target path, download nothing
Env MZLAB_BASE_URL replaces the release download URL (a mirror). Binaries: linux x86_64 and aarch64 (static), macOS arm64.
EOF
}

# The asset name of this machine (the fixed names of tools/note_release.py), or exit 3.
platform() {
	s=$(uname -s)
	m=$(uname -m)
	case "$s" in
	Linux)
		case "$m" in
		x86_64 | amd64) echo linux-x86_64 ;;
		aarch64 | arm64) echo linux-aarch64 ;;
		*) die 3 "no mzlab-cli binary for Linux $m (built: x86_64, aarch64); $FALLBACK" ;;
		esac
		;;
	Darwin)
		# A shell under Rosetta reports x86_64 on Apple silicon: the arm64 binary still runs natively there.
		if [ "$m" = x86_64 ] && [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || true)" = 1 ]; then m=arm64; fi
		case "$m" in
		arm64 | aarch64) echo macos-aarch64 ;;
		*) die 3 "no mzlab-cli binary for macOS $m (built: Apple silicon arm64 only); $FALLBACK" ;;
		esac
		;;
	*) die 3 "no mzlab-cli binary for $s $m (built: Linux x86_64/aarch64, macOS arm64); $FALLBACK" ;;
	esac
}

# fetch URL FILE: prints the HTTP status of the last response ("000" if there was none).
fetch() {
	if command -v curl >/dev/null 2>&1; then
		# Toward GitHub only HTTPS, redirects included; MZLAB_BASE_URL (a mirror, a test server) may be plain HTTP.
		if [ "$https_only" = 1 ]; then
			c=$(curl -sSL --proto '=https' --proto-redir '=https' --retry 2 -o "$2" -w '%{http_code}' "$1" 2>"$tmp/err") || :
		else
			c=$(curl -sSL --retry 2 -o "$2" -w '%{http_code}' "$1" 2>"$tmp/err") || :
		fi
		case "$c" in [0-9][0-9][0-9]) printf '%s' "$c" ;; *) printf '000' ;; esac
	else
		wget -S -O "$2" "$1" 2>"$tmp/err" || true
		awk '$1 ~ /^HTTP\// { c = $2 } END { printf "%s", (c == "" ? "000" : c) }' "$tmp/err"
	fi
}

# failed CODE URL: exit 6 unless the response was 200 (000 = no response: network, DNS, proxy).
failed() {
	case "$1" in
	200) ;;
	000) die 6 "download of $2 failed (no response): $(head -n 3 "$tmp/err" 2>/dev/null | tr '\n' ' ')" ;;
	*) die 6 "download of $2 failed: HTTP $1" ;;
	esac
}

sha256() {
	if command -v sha256sum >/dev/null 2>&1; then
		sha256sum "$1" | awk '{ print $1 }'
	else
		shasum -a 256 "$1" | awk '{ print $1 }'
	fi
}

main() {
	version=${MZLAB_VERSION:-}
	dir=${MZLAB_INSTALL_DIR:-}
	dry=0
	while [ $# -gt 0 ]; do
		case "$1" in
		--version) [ $# -ge 2 ] || die 2 "--version needs X.Y.Z"; version=$2; shift 2 ;;
		--dir) [ $# -ge 2 ] || die 2 "--dir needs a folder"; dir=$2; shift 2 ;;
		--dry-run) dry=1; shift ;;
		-h | --help) usage; exit 0 ;;
		*) usage >&2; die 2 "unknown argument: $1" ;;
		esac
	done
	version=${version#v}
	if [ -z "$dir" ]; then
		[ -n "${HOME:-}" ] || die 2 "HOME is not set: pass --dir DIR"
		dir="$HOME/.local/bin"
	fi

	plat=$(platform)
	asset="mzlab-cli-$plat"
	https_only=1
	if [ -n "${MZLAB_BASE_URL:-}" ]; then
		base=${MZLAB_BASE_URL%/}
		https_only=0
	elif [ -n "$version" ]; then
		base="https://github.com/$REPO/releases/download/v$version"
	else
		base="https://github.com/$REPO/releases/latest/download"
	fi
	say "platform $plat, asset $asset"
	say "from $base"
	say "into $dir/mzlab-cli"
	if [ "$dry" = 1 ]; then
		printf 'platform=%s\nasset=%s\nurl=%s/%s\nsums=%s/SHA256SUMS\ntarget=%s/mzlab-cli\n' \
			"$plat" "$asset" "$base" "$asset" "$base" "$dir"
		exit 0
	fi

	command -v curl >/dev/null 2>&1 || command -v wget >/dev/null 2>&1 || die 4 "neither curl nor wget is installed"
	command -v sha256sum >/dev/null 2>&1 || command -v shasum >/dev/null 2>&1 ||
		die 4 "neither sha256sum nor shasum is installed: the download cannot be verified"

	tmp=$(mktemp -d 2>/dev/null || mktemp -d -t mzlab)
	trap 'rm -rf "$tmp"' EXIT
	trap 'exit 130' INT TERM

	code=$(fetch "$base/$asset" "$tmp/$asset")
	[ "$code" != 404 ] || die 5 "the release has no asset $asset ($base/$asset: HTTP 404); a draft or unpublished release is not reachable at releases/latest; $FALLBACK"
	failed "$code" "$base/$asset"
	code=$(fetch "$base/SHA256SUMS" "$tmp/SHA256SUMS")
	[ "$code" != 404 ] || die 7 "the release has no SHA256SUMS ($base/SHA256SUMS: HTTP 404): $asset cannot be verified, nothing installed"
	failed "$code" "$base/SHA256SUMS"

	want=$(awk -v n="$asset" '{ f = $2; sub(/^\*/, "", f) } f == n { print tolower($1); exit }' "$tmp/SHA256SUMS")
	[ -n "$want" ] || die 7 "SHA256SUMS does not list $asset: nothing installed"
	got=$(sha256 "$tmp/$asset" | tr 'A-F' 'a-f')
	[ "$got" = "$want" ] || die 7 "checksum differs for $asset: SHA256SUMS says $want, the download is $got; nothing installed"
	say "SHA-256 $got matches SHA256SUMS"

	chmod 755 "$tmp/$asset"
	ver=$("$tmp/$asset" --version 2>&1) || die 8 "the downloaded $asset does not run on this machine: $ver"
	mkdir -p "$dir" 2>/dev/null || die 8 "cannot create $dir: pass --dir with a folder you can write"
	# Copy next to the target, then rename: an existing mzlab-cli is replaced only by a whole, verified file.
	cp "$tmp/$asset" "$dir/.mzlab-cli.tmp.$$" 2>/dev/null || die 8 "cannot write in $dir: pass --dir with a folder you can write"
	mv -f "$dir/.mzlab-cli.tmp.$$" "$dir/mzlab-cli" || { rm -f "$dir/.mzlab-cli.tmp.$$"; die 8 "cannot replace $dir/mzlab-cli"; }

	case ":${PATH:-}:" in
	*":$dir:"*) ;;
	*) say "$dir is not in PATH: add it (export PATH=\"$dir:\$PATH\") or call $dir/mzlab-cli" ;;
	esac
	printf '%s\n%s\n' "$dir/mzlab-cli" "$ver"
}

main "$@"
