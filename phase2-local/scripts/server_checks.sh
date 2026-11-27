#!/usr/bin/env bash
# Server checks on disposable containers bound to 127.0.0.1: postgres:17 (+ pgTAP), GoTrue and
# PostgREST on a private Docker network. Applies the plain-Postgres bootstrap and every migration
# in order, runs pgTAP and the opt-in server suites one at a time, and removes the containers.
# The JWT secret and passwords are generated here per run and never written to the repository.
# Run from phase2-local. Optional: CARGO_FLAGS (default "--locked --offline").
set -euo pipefail

cd "$(dirname "$0")/.."
repo=$(cd .. && pwd)
run=alm-checks-$$
net=$run-net
pg_port=${ALM_PG_PORT:-54329}
rest_port=${ALM_REST_PORT:-3000}
auth_port=${ALM_AUTH_PORT:-9999}
flags=${CARGO_FLAGS:---locked --offline}
# Docker Hub's official postgres image through its Amazon ECR Public mirror (no Docker Hub pull
# limit); PostgREST has no official mirror there, so every pull is retried once after a pause.
postgres_image=${POSTGRES_IMAGE:-public.ecr.aws/docker/library/postgres:17}
gotrue_image=${GOTRUE_IMAGE:-public.ecr.aws/supabase/gotrue:v2.198.0}
postgrest_image=${POSTGREST_IMAGE:-postgrest/postgrest:latest}

secret=$(openssl rand -hex 32)
pg_password=$(openssl rand -hex 16)
auth_password=$(openssl rand -hex 16)
rest_password=$(openssl rand -hex 16)
user_password=$(openssl rand -hex 12)

cleanup() {
  docker rm -f "$run-auth" "$run-rest" "$run-db" >/dev/null 2>&1 || true
  docker network rm "$net" >/dev/null 2>&1 || true
}
trap cleanup EXIT

pull() {
  docker pull -q "$1" >/dev/null || { echo "pull of $1 failed; retrying in 60s"; sleep 60; docker pull -q "$1" >/dev/null; }
}
for image in "$postgres_image" "$postgrest_image" "$gotrue_image"; do pull "$image"; done

docker network create "$net" >/dev/null
docker run -d --name "$run-db" --network "$net" -p "127.0.0.1:$pg_port:5432" \
  -e POSTGRES_PASSWORD="$pg_password" "$postgres_image" >/dev/null
for _ in $(seq 1 60); do
  docker exec "$run-db" pg_isready -U postgres -h 127.0.0.1 >/dev/null 2>&1 && break
  sleep 1
done
docker exec "$run-db" sh -c 'apt-get update -qq >/dev/null && apt-get install -y -qq postgresql-17-pgtap >/dev/null'

psql_db() { docker exec -i "$run-db" psql -X -q -v ON_ERROR_STOP=1 -U postgres -d postgres "$@"; }

psql_db < "$repo/supabase/tests/plain_postgres/bootstrap.sql"
for migration in "$repo"/supabase/migrations/*.sql; do
  echo "migration $(basename "$migration")"
  psql_db < "$migration"
done
psql_db < "$repo/supabase/tests/plain_postgres/gotrue.sql"
psql_db -v auth="$auth_password" -v rest="$rest_password" <<'SQL'
alter role supabase_auth_admin password :'auth';
create role authenticator login noinherit password :'rest';
grant anon, authenticated, service_role to authenticator;
SQL

echo "== pgTAP"
failed=0
for test in "$repo"/supabase/tests/database/*.test.sql; do
  out=$(psql_db -t -A < "$test" 2>&1) || { echo "$out"; failed=1; continue; }
  planned=$(printf '%s\n' "$out" | sed -n 's/^1\.\.\([0-9]*\)$/\1/p')
  passed=$(printf '%s\n' "$out" | grep -c '^ok ' || true)
  if printf '%s\n' "$out" | grep -qE '^not ok|^# Looks like' || [ "$planned" != "$passed" ]; then
    echo "$out"
    failed=1
  fi
  echo "$(basename "$test"): $passed/$planned"
done
[ "$failed" = 0 ]

docker run -d --name "$run-rest" --network "$net" -p "127.0.0.1:$rest_port:3000" \
  -e PGRST_DB_URI="postgres://authenticator:$rest_password@$run-db:5432/postgres" \
  -e PGRST_DB_SCHEMAS=public -e PGRST_DB_ANON_ROLE=anon \
  -e PGRST_JWT_SECRET="$secret" "$postgrest_image" >/dev/null
docker run -d --name "$run-auth" --network "$net" -p "127.0.0.1:$auth_port:9999" \
  -e GOTRUE_API_HOST=0.0.0.0 -e PORT=9999 -e API_EXTERNAL_URL="http://127.0.0.1:$auth_port" \
  -e GOTRUE_SITE_URL="http://127.0.0.1" -e GOTRUE_DB_DRIVER=postgres -e DB_NAMESPACE=auth \
  -e GOTRUE_DB_DATABASE_URL="postgres://supabase_auth_admin:$auth_password@$run-db:5432/postgres?search_path=auth" \
  -e GOTRUE_JWT_SECRET="$secret" -e GOTRUE_JWT_EXP=3600 -e GOTRUE_JWT_AUD=authenticated \
  -e GOTRUE_JWT_DEFAULT_GROUP_NAME=authenticated -e GOTRUE_JWT_ADMIN_ROLES=service_role \
  -e GOTRUE_EXTERNAL_EMAIL_ENABLED=true -e GOTRUE_MAILER_AUTOCONFIRM=true \
  -e GOTRUE_DISABLE_SIGNUP=false "$gotrue_image" >/dev/null
for _ in $(seq 1 60); do
  curl -fsS "http://127.0.0.1:$rest_port/" >/dev/null 2>&1 \
    && curl -fsS "http://127.0.0.1:$auth_port/health" >/dev/null 2>&1 && break
  sleep 1
done
curl -fsS "http://127.0.0.1:$auth_port/health" >/dev/null || { docker logs "$run-auth"; exit 1; }
curl -fsS "http://127.0.0.1:$rest_port/" >/dev/null || { docker logs "$run-rest"; exit 1; }

export ALM_PG_URL="postgres://postgres:$pg_password@127.0.0.1:$pg_port/postgres"
export ALM_REST_URL="http://127.0.0.1:$rest_port"
export ALM_GOTRUE_URL="http://127.0.0.1:$auth_port"
export ALM_JWT_SECRET="$secret"
export ALM_SKIP_PERF=1

suite() {
  echo "== $*"
  # shellcheck disable=SC2086
  cargo test $flags "$@" -- --test-threads=1
}

suite -p alm-sync --test conformance_postgres
suite -p alm-sync --test permission_matrix
suite -p alm-sync --test differential_postgres
# shellcheck disable=SC2086
echo "== -p alm-sync --test randomized" && cargo test $flags -p alm-sync --test randomized -- --nocapture
suite -p alm-sync --test postgres_authority
suite -p alm-sync --test postgres_replay
suite -p alm-sync --test rest_authority
suite -p alm-sync --test gotrue_session

# The app's REST end to end: one account holding the Owner and the Manager token.
mint() {
  python3 - "$secret" "$1" <<'PY'
import base64, hashlib, hmac, json, sys
b64 = lambda b: base64.urlsafe_b64encode(b).rstrip(b"=").decode()
claims = {"role": "authenticated", "aud": "authenticated", "sub": "alm-test-app",
          "exp": 4102444800, "app_metadata": {"role": sys.argv[2]}}
signed = b64(b'{"alg":"HS256","typ":"JWT"}') + "." + b64(json.dumps(claims, separators=(",", ":")).encode())
print(signed + "." + b64(hmac.new(sys.argv[1].encode(), signed.encode(), hashlib.sha256).digest()))
PY
}
seed() { ALM_RESET_ONLY=1 suite -p alm-sync --test rest_authority reset_and_seed_for_the_app; }
seed
ALM_JWT_OWNER=$(mint owner) ALM_JWT_MANAGER=$(mint manager) \
  suite -p alm-app --lib rest_e2e::two_devices_through_rest_see_the_same_posted_rows
ALM_JWT_OWNER=$(mint owner) ALM_JWT_MANAGER=$(mint manager) \
  suite -p alm-app --lib rest_e2e::owner_review_in_rest_mode_names_item_lines_from_the_pulled_catalog

# Real GoTrue accounts on the device: sign-in, sign-out purge, relaunch.
seed
service=$(python3 - "$secret" <<'PY'
import base64, hashlib, hmac, json, sys, time
b64 = lambda b: base64.urlsafe_b64encode(b).rstrip(b"=").decode()
claims = {"role": "service_role", "exp": int(time.time()) + 600}
signed = b64(b'{"alg":"HS256","typ":"JWT"}') + "." + b64(json.dumps(claims, separators=(",", ":")).encode())
print(signed + "." + b64(hmac.new(sys.argv[1].encode(), signed.encode(), hashlib.sha256).digest()))
PY
)
for role in owner manager; do
  curl -fsS -o /dev/null -X POST "$ALM_GOTRUE_URL/admin/users" \
    -H "Authorization: Bearer $service" -H "Content-Type: application/json" \
    -d "{\"email\":\"$role@alm.test\",\"password\":\"$user_password\",\"email_confirm\":true,\"app_metadata\":{\"role\":\"$role\"}}"
done
ALM_OWNER_EMAIL=owner@alm.test ALM_MANAGER_EMAIL=manager@alm.test ALM_TEST_PASSWORD="$user_password" \
  suite -p alm-app --lib rest_e2e::people_sign_in_with_their_own_accounts_and_the_role_comes_from_the_token
# DEC-034: a signed-in Manager works the shop through REST without costs (on a fresh seed).
seed
ALM_OWNER_EMAIL=owner@alm.test ALM_MANAGER_EMAIL=manager@alm.test ALM_TEST_PASSWORD="$user_password" \
  suite -p alm-app --lib rest_e2e::a_signed_in_manager_works_the_shop_through_rest_without_costs
echo "server checks passed"
