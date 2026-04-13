#!/usr/bin/env bash
#
# Riven uninstaller.
#
# Removes ~/.riven and strips the PATH source line from shell rc files.

set -euo pipefail

RIVEN_HOME="${RIVEN_HOME:-$HOME/.riven}"

if [ -t 1 ]; then
  BOLD="$(printf '\033[1m')"; GREEN="$(printf '\033[32m')"
  YELLOW="$(printf '\033[33m')"; RESET="$(printf '\033[0m')"
else
  BOLD=""; GREEN=""; YELLOW=""; RESET=""
fi

SOURCE_LINE='. "$HOME/.riven/env"'
COMMENT_LINE='# Added by the Riven installer'

strip_rc() {
  local rc="$1"
  [ -f "$rc" ] || return 0
  if ! grep -Fq "$SOURCE_LINE" "$rc" 2>/dev/null; then
    return 0
  fi
  local tmp
  tmp="$(mktemp)"
  # Delete the comment line (if present) and the source line.
  grep -Fv "$SOURCE_LINE" "$rc" | grep -Fv "$COMMENT_LINE" > "$tmp"
  mv "$tmp" "$rc"
  echo "${GREEN}${BOLD} ✓${RESET} cleaned $rc"
}

for rc in "$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.zshrc" "$HOME/.profile"; do
  strip_rc "$rc"
done

if [ -d "$RIVEN_HOME" ]; then
  echo "${YELLOW}${BOLD} !${RESET} removing $RIVEN_HOME"
  rm -rf "$RIVEN_HOME"
  echo "${GREEN}${BOLD} ✓${RESET} removed $RIVEN_HOME"
else
  echo "${YELLOW}${BOLD} !${RESET} $RIVEN_HOME does not exist"
fi

echo
echo "${GREEN}${BOLD}Riven uninstalled.${RESET} Open a new shell to refresh PATH."
