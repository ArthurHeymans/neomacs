#!/usr/bin/env bash

# Shared, read-only Mach-O inspection helpers for macOS packaging scripts.
# The caller owns tool availability checks and error policy.

is_macho() {
  file -b "$1" 2>/dev/null | grep -q 'Mach-O'
}

macho_dependency_paths() {
  local otool_command="$1"
  local image="$2"

  "$otool_command" -l "$image" 2>/dev/null | awk '
    $1 == "cmd" && $2 ~ /^(LC_LOAD_DYLIB|LC_LOAD_WEAK_DYLIB|LC_REEXPORT_DYLIB|LC_LOAD_UPWARD_DYLIB|LC_LAZY_LOAD_DYLIB)$/ {
      load_command = 1
      next
    }
    load_command && $1 == "name" {
      sub(/^[[:space:]]*name[[:space:]]+/, "")
      sub(/[[:space:]]+\(offset[[:space:]]+[0-9]+\)$/, "")
      print
      load_command = 0
    }
  '
}
