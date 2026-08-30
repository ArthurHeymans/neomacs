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

# TWO root lists, because the bundle asks two different questions and one list
# answers them wrongly.
#
# NESTED roots are the subtrees codesign's V2 resource rules mark nested
# (Frameworks|SharedFrameworks|PlugIns|Plug-ins|XPCServices|Helpers|MacOS...).
# EVERY file under them is treated as nested code and must carry its own
# signature -- that is why the portable dump beside the executable had to be
# signed rather than skipped for not being Mach-O.
macos_bundle_nested_roots() {
  printf '%s\n' MacOS Frameworks Helpers PlugIns
}

# MODULE roots hold loadable code that is NOT nested: a subdirectory of a
# nested root must be a real bundle, and a plug-in directory is not one, so the
# GStreamer plug-ins and GIO modules live under Resources instead.  Files here
# are sealed as resources by the bundle signature, so they must NOT be signed
# one by one -- Contents/Resources/neomacs alone is ~4500 Lisp and etc files,
# and codesign --verify --strict fails on a text file.  Only the Mach-O images
# here need their own signature, because dlopen under the hardened runtime
# requires one.
macos_bundle_module_roots() {
  printf '%s\n' Resources
}

# Everything that must be WALKED when relocating load commands or auditing
# dependencies: both kinds carry Mach-O images we own.
macos_bundle_scan_roots() {
  macos_bundle_nested_roots
  macos_bundle_module_roots
}
