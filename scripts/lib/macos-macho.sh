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

# The bundle subtrees that carry code we own: executables, vendored libraries,
# helpers, and the loadable modules under Resources.  ONE list, because it is
# consumed by vendoring (copy, drop, relocate), signing, and the audit -- and a
# root added to one but not the others silently half-processes the bundle.
# That is exactly what happened when the GStreamer plug-ins moved from PlugIns
# to Resources: the audit walked Resources, the vendorer did not, and 2934
# dependencies went unrelocated while the counts looked merely smaller.
#
# Resources is included because loadable modules live there: codesign's V2
# resource rules mark Frameworks|PlugIns|MacOS|Helpers NESTED, so a SUBDIRECTORY
# of those must be a real bundle, and a plug-in directory is not one.
macos_bundle_code_roots() {
  printf '%s\n' MacOS Frameworks Helpers PlugIns Resources
}
