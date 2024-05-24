set windows-shell := ["nu.exe", "-c"]
set shell := ["nu", "-c"]

root_path := absolute_path('')
scripts_path := absolute_path('scripts')
cli_path := absolute_path('src/cli')
probe_path := absolute_path('src/probe')
cli_config_path := absolute_path('src/flake/modules/config.toml')

default: prepare

prepare:
  cd "{{root_path}}"; poetry install --no-root
  cd "{{probe_path}}"; poetry install --no-root
  docker compose down -v
  docker compose up -d
  sleep 3sec
  cd "{{cli_path}}"; sqlx migrate run

ci:
  cd "{{root_path}}"; poetry install --no-root
  cd "{{probe_path}}"; poetry install --no-root

format:
  cd "{{root_path}}"; cargo fmt --all
  yapf --recursive --in-place --parallel "{{root_path}}"
  prettier --write "{{root_path}}"
  shfmt --write "{{root_path}}"

lint:
  cd "{{root_path}}"; cargo clippy -- -D warnings
  ruff check "{{root_path}}"
  cd "{{probe_path}}"; pyright .
  glob '/home/haras/src/pidgeon/scripts/*' | each { |i| shellcheck $i } | str join "\n"
  prettier --check "{{root_path}}"

test:
  cd "{{root_path}}"; cargo test

build:
  cd "{{root_path}}"; cargo build --release

run *args:
  cd "{{cli_path}}"; cargo run -- --config "{{cli_config_path}}" {{args}}

probe *args:
  cd "{{probe_path}}"; python ./probe/main.py {{args}}
