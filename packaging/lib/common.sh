#!/usr/bin/env bash
# Shared helpers for packaging/*/pack.sh. Source from repo packaging scripts only.

packaging_root() {
  local lib
  lib="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  (cd "$lib/../.." && pwd)
}

minimix_version() {
  if [[ -n "${MINIMIX_VERSION:-}" ]]; then
    printf '%s\n' "$MINIMIX_VERSION"
    return
  fi
  if [[ -n "${GITHUB_REF_NAME:-}" && "${GITHUB_REF_NAME}" == v* ]]; then
    printf '%s\n' "${GITHUB_REF_NAME#v}"
    return
  fi
  local tag
  tag="$(git -C "$(packaging_root)" describe --tags --exact-match 2>/dev/null || true)"
  if [[ "$tag" == v* ]]; then
    printf '%s\n' "${tag#v}"
    return
  fi
  printf '0.0.0-dev+%s\n' "$(git -C "$(packaging_root)" rev-parse --short HEAD)"
}

spec_field() {
  local platform="$1"
  local field="$2"
  local root
  root="$(packaging_root)"
  python3 - "$root/packaging/package-spec.json" "$platform" "$field" <<'PY'
import json, sys
spec = json.load(open(sys.argv[1]))
platform = sys.argv[2]
field = sys.argv[3]
row = spec["platforms"][platform]
value = row[field]
if isinstance(value, list):
    print(",".join(value))
else:
    print(value)
PY
}

artifact_name() {
  local version="$1"
  local platform="$2"
  local archive
  archive="$(spec_field "$platform" archive)"
  printf 'minimix-%s-%s.%s\n' "$version" "$platform" "$archive"
}

stage_dir_for() {
  local version="$1"
  local platform="$2"
  local root
  root="$(packaging_root)"
  printf '%s/dist/stage/minimix-%s-%s\n' "$root" "$version" "$platform"
}

write_notes() {
  local platform="$1"
  local dest="$2"
  local signing components notes
  signing="$(spec_field "$platform" signing)"
  components="$(spec_field "$platform" components)"
  notes="$(spec_field "$platform" notes)"
  {
    printf 'platform: %s\n' "$platform"
    printf 'signing: %s\n' "$signing"
    printf 'components: %s\n' "$components"
    printf '\n%s\n' "$notes"
  } >"$dest"
}

make_archive() {
  local platform="$1"
  local stage_parent="$2"
  local stage_name="$3"
  local out_path="$4"
  local archive
  archive="$(spec_field "$platform" archive)"
  mkdir -p "$(dirname "$out_path")"
  rm -f "$out_path"
  case "$archive" in
    tar.gz)
      tar -C "$stage_parent" -czf "$out_path" "$stage_name"
      ;;
    zip)
      (cd "$stage_parent" && zip -qr "$out_path" "$stage_name")
      ;;
    *)
      echo "unknown archive type: $archive" >&2
      return 1
      ;;
  esac
  printf '%s\n' "$out_path"
}
