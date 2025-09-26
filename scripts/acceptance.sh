#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT_DIR"

: "${BIND_ADDR:=127.0.0.1:5005}"
: "${LEX_DB_PATH:=vendor/irish-lex-db/lexicon.sqlite}"
: "${TERM_DB_PATH:=vendor/irish-lex-db/terminology.sqlite}"
: "${ACCEPT_REMOTE:=0}"

robust_get() {
  local path="$1"; shift
  local jqf="${1:-.}"; shift || true
  echo "[acceptance] GET $path"
  rm -f /tmp/resp.json || true
  local code;
  code=$(curl -s -H 'Accept-Encoding: identity' -o /tmp/resp.json -w '%{http_code}' "http://$BIND_ADDR$path" || true)
  echo "[acceptance]  status=$code"
  if [ ! -f /tmp/resp.json ]; then
    echo "[acceptance]  (no body)"; return 0
  fi
  if [ "$code" = "200" ]; then
    jq "$jqf" </tmp/resp.json || (echo "[acceptance]  (jq parse error, showing head)"; head -c 200 /tmp/resp.json; echo)
  else
    head -c 200 /tmp/resp.json; echo
  fi
}

robust_post_json() {
  local path="$1"; shift
  local body="$1"; shift
  local jqf="${1:-.}"; shift || true
  echo "[acceptance] POST $path"
  rm -f /tmp/resp.json || true
  local code;
  code=$(curl -s -H 'Accept-Encoding: identity' -H 'Content-Type: application/json' -o /tmp/resp.json -w '%{http_code}' -X POST "http://$BIND_ADDR$path" -d "$body" || true)
  echo "[acceptance]  status=$code"
  if [ ! -f /tmp/resp.json ]; then
    echo "[acceptance]  (no body)"; return 0
  fi
  if [ "$code" = "200" ]; then
    jq "$jqf" </tmp/resp.json || (echo "[acceptance]  (jq parse error, showing head)"; head -c 200 /tmp/resp.json; echo)
  else
    head -c 200 /tmp/resp.json; echo
  fi
}

echo "[acceptance] building (offline release)"
if [ "$ACCEPT_REMOTE" != "1" ]; then
  SQLX_OFFLINE=1 cargo build --release >/dev/null
fi

if [ "$ACCEPT_REMOTE" != "1" ]; then
  echo "[acceptance] starting server at $BIND_ADDR"
  (
    BIND_ADDR="$BIND_ADDR" \
    LEX_DB_PATH="$LEX_DB_PATH" \
    TERM_DB_PATH="$TERM_DB_PATH" \
    SQLX_OFFLINE=1 \
    ./target/release/focloireacht-server
  ) &
  SERVER_PID=$!
  cleanup() {
    kill $SERVER_PID >/dev/null 2>&1 || true
  }
  trap cleanup EXIT
else
  echo "[acceptance] remote mode; assuming server at $BIND_ADDR"
fi

echo "[acceptance] waiting for server..."
for i in {1..100}; do
  code=$(curl -s -o /dev/null -w '%{http_code}' "http://$BIND_ADDR/health" || true)
  if [ "$code" = "200" ]; then
    break
  fi
  sleep 0.2
done

# Health
robust_get "/health" '.'

# Meta (summary)
robust_get "/meta" '{lex:{schema_version:.lex.schema_version, build_time:.lex.build_time, sources:(.lex.sources|length)}, term:{schema_version:.term.schema_version, build_time:.term.build_time, sources:(.term.sources|length)}}'

# Lex lookups
robust_get "/lex/entry?lemma=achrann&limit=2" '.[0]'
robust_get "/lex/by-variant?form=achrainn&limit=2" '.'
robust_post_json "/lex/batch" '{"lemmas":["achrann"],"variants":["achrainn"],"limit":2}' '{lemmas: (.lemmas|to_entries[0]), variants: (.variants|to_entries[0])}'

# Term pairs
robust_get "/term/en2ga?term=network&limit=5" '{query, matches_len:(.matches|length), sample:(.matches[0]|{ga:.ga.term,en:.en.term})}'
# Try a GA sample spelling; accept empty
robust_get "/term/ga2en?term=l%C3%ADonra&limit=5" '{query, matches_len:(.matches|length), sample:(.matches[0]|{ga:.ga.term,en:.en.term})}'

# Domains summary
robust_get "/term/domains" '{count:(.domains|length), sample:(.domains[0])}'

# Validate sample
robust_get "/term/validate?term=network&lang=en" '.'

# Batch terms
robust_post_json "/term/batch" '{"lang":"en","terms":["network","account"],"limit":5}' '[.[0].query, {matches_len:(.[0].matches|length)}]'

echo "[acceptance] OK"

