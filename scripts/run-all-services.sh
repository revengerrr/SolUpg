#!/usr/bin/env bash
# run-all-services.sh — bring up the SolUPG stack via docker compose.
#
# Wraps docker compose so you don't need to remember -f path and ordering.
#
# Usage:
#   scripts/run-all-services.sh up          # build + start detached
#   scripts/run-all-services.sh down        # stop everything, keep volumes
#   scripts/run-all-services.sh logs        # tail all service logs
#   scripts/run-all-services.sh logs api    # tail one service
#   scripts/run-all-services.sh status      # ps-style status
#   scripts/run-all-services.sh rebuild     # down + build + up
#   scripts/run-all-services.sh nuke        # down + volume removal (DANGER)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="$(cd "${SCRIPT_DIR}/../services" && pwd)/docker-compose.yml"

compose() {
  docker compose -f "${COMPOSE_FILE}" "$@"
}

cmd="${1:-up}"
shift || true

case "${cmd}" in
  up)
    compose up -d --build
    compose ps
    echo
    echo "Service endpoints:"
    echo "  routing-engine    http://localhost:3000"
    echo "  directory-service http://localhost:3001"
    echo "  api-gateway       http://localhost:3002"
    echo "  clearing-engine   http://localhost:3003"
    echo "  monitoring        http://localhost:3004"
    ;;
  down)
    compose down
    ;;
  logs)
    if [[ $# -gt 0 ]]; then
      compose logs -f "$@"
    else
      compose logs -f
    fi
    ;;
  status|ps)
    compose ps
    ;;
  rebuild)
    compose down
    compose build --no-cache
    compose up -d
    compose ps
    ;;
  nuke)
    echo "This will remove the Postgres volume. Type 'nuke' to confirm:"
    read -r ans
    if [[ "${ans}" == "nuke" ]]; then
      compose down -v
      echo "Volumes removed."
    else
      echo "Aborted."
    fi
    ;;
  *)
    echo "Unknown command: ${cmd}"
    echo "See file header for usage."
    exit 2
    ;;
esac
