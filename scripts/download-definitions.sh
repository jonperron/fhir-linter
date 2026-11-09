#!/usr/bin/env bash
# download-definitions.sh — Fetch and verify FHIR R5 definition artifacts.
#
# Usage: scripts/download-definitions.sh [--force]
#   --force   Re-download even if files already exist.
#
# Files are placed in definitions/r5/. The SHA-256 checksum is verified before
# unpacking. The downloaded archive is removed after extraction.
#
# SECURITY NOTES:
#   - TLS 1.2+ is enforced via --tlsv1.2.
#   - SHA-256 is verified before extraction (supply-chain integrity).
#   - The archive comes from hl7.org and its hash is pinned below; unzip
#     strips leading "/" and "../" components of entry names by default.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
OUT_DIR="${ROOT_DIR}/definitions/r5"

FORCE=false
for arg in "$@"; do
    if [[ "${arg}" == "--force" ]]; then
        FORCE=true
    fi
done

# FHIR R5 release artifact
DEFINITIONS_URL="https://hl7.org/fhir/R5/definitions.json.zip"

# SHA-256 checksum (update when a new FHIR R5 patch is released).
# Obtain the correct value by running:
#   curl -fsSL <URL> | sha256sum
DEFINITIONS_SHA256="df0d7259b4a8741d59f4971d96dd486423ecbd414c7060e9dc006ae3c3209c0c"

# ---------------------------------------------------------------------------

require_cmd() {
    if ! command -v "$1" &>/dev/null; then
        echo "ERROR: '$1' is required but not found in PATH." >&2
        exit 2
    fi
}

require_cmd curl
require_cmd sha256sum
require_cmd unzip

mkdir -p "${OUT_DIR}"

archive="${OUT_DIR}/$(basename "${DEFINITIONS_URL}")"
sentinel="${archive%.zip}.sentinel"

if [[ "${FORCE}" == false && -f "${sentinel}" ]]; then
    echo "[skip] $(basename "${archive}") already downloaded and verified."
    exit 0
fi

echo "[fetch] ${DEFINITIONS_URL}"
# --tlsv1.2   : enforce TLS 1.2 minimum (OWASP A02 — Cryptographic Failures)
# --location  : follow redirects (hl7.org uses HTTP→HTTPS redirect)
# --fail      : non-200 responses abort with exit code 22
curl --fail --silent --show-error --location --tlsv1.2 --output "${archive}" "${DEFINITIONS_URL}"

echo "[verify] $(basename "${archive}")"
actual_sha256="$(sha256sum "${archive}" | awk '{print $1}')"
if [[ "${actual_sha256}" != "${DEFINITIONS_SHA256}" ]]; then
    echo "ERROR: SHA-256 mismatch for $(basename "${archive}")." >&2
    echo "  expected: ${DEFINITIONS_SHA256}" >&2
    echo "  actual:   ${actual_sha256}" >&2
    rm -f "${archive}"
    exit 1
fi

echo "[unzip] $(basename "${archive}")"
unzip -q -o "${archive}" -d "${OUT_DIR}"
rm -f "${archive}"
touch "${sentinel}"

echo ""
echo "FHIR R5 definitions ready in ${OUT_DIR}"
