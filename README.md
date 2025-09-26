# focloireacht-server

Small production-ready Rust service exposing exact-match lexicon and terminology queries over two prebuilt SQLite databases.

## Stack

- Rust stable
- axum + tokio
- sqlx (SQLite, prepared statements)
- serde_json

## Layout

```
focloireacht-server/
  Cargo.toml
  .sqlx/
  src/
    main.rs
    config.rs
    db.rs
    routes/
      health.rs
      lex.rs
      term.rs
      meta.rs
    models/
      lex.rs
      term.rs
      meta.rs
```

## Configuration

- `LEX_DB_PATH` (default: `./data/lexicon.sqlite`)
- `TERM_DB_PATH` (default: `./data/terminology.sqlite`)
- `BIND_ADDR` (default: `127.0.0.1:5005`)

Connections are read-only: `sqlite:PATH?mode=ro`.

## Run (local)

```bash
LEX_DB_PATH=vendor/irish-lex-db/lexicon.sqlite \
TERM_DB_PATH=vendor/irish-lex-db/terminology.sqlite \
SQLX_OFFLINE=1 \
cargo run --release
```

## Docker

Build:

```bash
make build IMAGE=caffalaughrey/focloireacht-server TAG=latest
```

Run:

```bash
docker run --rm -p 5005:5005 \
  -e BIND_ADDR=0.0.0.0:5005 \
  -e LEX_DB_PATH=/data/lexicon.sqlite \
  -e TERM_DB_PATH=/data/terminology.sqlite \
  -v $(pwd)/vendor/irish-lex-db:/data:ro \
  caffalaughrey/focloireacht-server:latest
```

Compose (join external gateway network), similar to `gaelspell-server`:

```yaml
services:
  focloireacht:
    image: caffalaughrey/focloireacht-server:latest
    environment:
      - BIND_ADDR=0.0.0.0:5005
      - LEX_DB_PATH=/data/lexicon.sqlite
      - TERM_DB_PATH=/data/terminology.sqlite
    volumes:
      - ./vendor/irish-lex-db:/data:ro
    ports:
      - "5005:5005"
    networks:
      - gateway_net
networks:
  gateway_net:
    external: true
```

## Endpoints

- `GET /health` → `{ "status": "ok" }`
- `GET /meta` → DB meta and sources projection.
- `GET /lex/entry?lemma=<lookup_key>&limit=<n>`
- `GET /lex/by-variant?form=<form>&limit=<n>`
- `POST /lex/batch` → `{ lemmas: {key: [...]}, variants: {form: [...]}}`
- `GET /term/en2ga?term=<en>&domain=<label>&limit=<n>`
- `GET /term/ga2en?term=<ga>&domain=<label>&limit=<n>`
- `GET /term/domains` → `{ domains: [ {label, count}, ... ] }`
- `GET /term/validate?term=<ga|en>&lang=<ga|en>&domain=<label>` → `{ valid: bool }`
- `POST /term/batch` → array of en2ga/ga2en-like results.

All matching is exact on stored keys/labels. Upstream is responsible for normalization and morphology.

## Notes

- No writes; read-only SQLite.
- Input validation: non-empty, length ≤ 128; limit default 5, max 50.
- Errors: 400 on bad params; 500 on DB errors.

## Related

- Sister container for GaelSpell: [gaelspell-server](https://github.com/caffalaughrey/gaelspell-server/)
