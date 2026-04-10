#!/usr/bin/env bash
# seed-db.sh — apply migrations and insert fixture data into the local DB.
#
# Preconditions:
#   - docker compose postgres is running (scripts/run-all-services.sh up)
#   - psql CLI is installed on the host, OR this script will run psql
#     inside the solupg-postgres container if psql is not on PATH.
#
# Usage:
#   scripts/seed-db.sh                 # apply migrations + fixtures
#   scripts/seed-db.sh --migrations    # migrations only, no fixtures
#   scripts/seed-db.sh --fixtures      # fixtures only, skip migrations
#   scripts/seed-db.sh --reset         # drop + recreate solupg DB, then full

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MIGRATIONS_DIR="$(cd "${SCRIPT_DIR}/../services/migrations" && pwd)"

DB_HOST="${PGHOST:-localhost}"
DB_PORT="${PGPORT:-5433}"
DB_USER="${PGUSER:-solupg}"
DB_PASS="${PGPASSWORD:-solupg_dev}"
DB_NAME="${PGDATABASE:-solupg}"

MODE="all"
if [[ $# -gt 0 ]]; then
  case "$1" in
    --migrations) MODE="migrations" ;;
    --fixtures)   MODE="fixtures" ;;
    --reset)      MODE="reset" ;;
    *) echo "Unknown flag: $1"; exit 2 ;;
  esac
fi

# Resolve a psql invocation. Prefer host CLI; fall back to container exec.
if command -v psql >/dev/null 2>&1; then
  PSQL=(env PGPASSWORD="${DB_PASS}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d "${DB_NAME}")
  PSQL_ADMIN=(env PGPASSWORD="${DB_PASS}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d postgres)
else
  echo "psql not on PATH; using docker exec against solupg-postgres"
  PSQL=(docker exec -i -e PGPASSWORD="${DB_PASS}" solupg-postgres psql -U "${DB_USER}" -d "${DB_NAME}")
  PSQL_ADMIN=(docker exec -i -e PGPASSWORD="${DB_PASS}" solupg-postgres psql -U "${DB_USER}" -d postgres)
fi

reset_db() {
  echo "==> Dropping and recreating ${DB_NAME}"
  "${PSQL_ADMIN[@]}" -c "DROP DATABASE IF EXISTS ${DB_NAME};"
  "${PSQL_ADMIN[@]}" -c "CREATE DATABASE ${DB_NAME} OWNER ${DB_USER};"
}

run_migrations() {
  echo "==> Applying migrations from ${MIGRATIONS_DIR}"
  if [[ ! -d "${MIGRATIONS_DIR}" ]]; then
    echo "ERROR: migrations dir not found"
    exit 1
  fi
  shopt -s nullglob
  local files=("${MIGRATIONS_DIR}"/*.sql)
  shopt -u nullglob
  if [[ ${#files[@]} -eq 0 ]]; then
    echo "    No *.sql files in migrations dir. Skipping."
    return
  fi
  for f in "${files[@]}"; do
    echo "    - $(basename "${f}")"
    "${PSQL[@]}" -v ON_ERROR_STOP=1 -f "${f}" >/dev/null
  done
}

insert_fixtures() {
  echo "==> Inserting fixtures"
  "${PSQL[@]}" -v ON_ERROR_STOP=1 <<'SQL'
-- Idempotent fixture block. Safe to re-run.
DO $$
BEGIN
  -- Demo merchant (expand as services require).
  IF NOT EXISTS (SELECT 1 FROM pg_tables WHERE tablename = 'merchants') THEN
    RAISE NOTICE 'merchants table missing — skip merchant fixture';
  END IF;
END$$;
SQL
  echo "    Fixtures applied (no-op placeholder — extend as tables land)."
}

case "${MODE}" in
  all)
    run_migrations
    insert_fixtures
    ;;
  migrations)
    run_migrations
    ;;
  fixtures)
    insert_fixtures
    ;;
  reset)
    reset_db
    run_migrations
    insert_fixtures
    ;;
esac

echo
echo "==> Done."
