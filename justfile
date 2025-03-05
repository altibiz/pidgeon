set windows-shell := ["nu.exe", "-c"]
set shell := ["nu", "-c"]

root := absolute_path('')

default:
    @just --choose

prepare:
    dvc pull
    docker compose down -v
    docker compose up -d
    loop { \
      try { \
        let timescale_container_id = (docker compose ps --format json \
          | lines \
          | each { $in | from json } \
          | filter { $in.Image | str starts-with "timescale" } \
          | first \
          | get id); \
        docker exec $timescale_container_id pg_isready --host localhost; \
        break \
      } catch { \
        sleep 1sec; \
        continue \
      } \
    }
    cd '{{ root }}/src/cli'; cargo sqlx migrate run

lfs:
    dvc add {{ root }}/assets/measurements/*.csv
    dvc push

run *args:
    cd '{{ root }}/src/cli'; cargo run -- --config '{{ root }}/assets/pidgeon/config.toml' {{ args }}

probe-client *args:
    pidgeon-probe client \
      --config '{{ root }}/assets/pidgeon/config.toml' \
      {{ args }}

probe-server *args:
    pidgeon-probe server \
      --config '{{ root }}/assets/pidgeon/config.toml' \
      --measurements '{{ root }}/assets/measurements' \
      {{ args }}

test:
    cd '{{ root }}'; cargo test

format:
    cd '{{ root }}'; just --unstable --fmt
    prettier --write '{{ root }}'
    nixpkgs-fmt '{{ root }}'
    shfmt --write '{{ root }}'
    yapf --recursive --in-place --parallel '{{ root }}'
    cd '{{ root }}'; cargo fmt --all

lint:
    cd '{{ root }}'; just --unstable --fmt --check
    prettier --check '{{ root }}'
    cspell lint '{{ root }}' --no-progress
    nixpkgs-fmt '{{ root }}' --check
    glob '{{ root }}/scripts/**/*.sh' | each { |i| shellcheck $i } | str join "\n"
    ruff check '{{ root }}'
    pyright '{{ root }}'
    cd '{{ root }}'; $env.DATABASE_URL = null; cargo clippy -- -D warnings

upgrade:
    nix flake update
    cargo upgrade

docs:
    rm -rf '{{ root }}/artifacts'
    mkdir '{{ root }}/artifacts'
    cd '{{ root }}'; cargo doc --no-deps
    cd '{{ root }}/docs/en'; mdbook build
    cd '{{ root }}/docs/hr'; mdbook build
    mv '{{ root }}/target/doc' '{{ root }}/artifacts/code'
    mv '{{ root }}/docs/en/book' '{{ root }}/artifacts/en'
    mv '{{ root }}/docs/hr/book' '{{ root }}/artifacts/hr'
    cp '{{ root }}/docs/index.html' '{{ root }}/artifacts'

raspberryPi4 *args:
    {{ root }}/scripts/flake/raspberryPi4.nu {{ args }}
