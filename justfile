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
    dvc add {{ root }}/assets/measurements/*.sql
    dvc push

run *args:
    cd '{{ root }}/src/cli'; cargo run -- --config '{{ root }}/assets/pidgeon/config.toml' {{ args }}

probe *args:
    cd '{{ root }}/scripts/probe'; \
      $env.PIDGEON_PROBE_ENV = 'development'; \
      python -m probe.main {{ args }}

format:
    cd '{{ root }}'; just --unstable --fmt
    prettier --write '{{ root }}'
    cd '{{ root }}'; cargo fmt --all
    yapf --recursive --in-place --parallel '{{ root }}'
    shfmt --write '{{ root }}'
    nixpkgs-fmt '{{ root }}'

lint:
    cd '{{ root }}'; just --unstable --fmt --check
    nixpkgs-fmt '{{ root }}' --check
    prettier --check '{{ root }}'
    cspell lint '{{ root }}' --no-progress
    cd '{{ root }}'; cargo clippy -- -D warnings
    glob '{{ root }}/scripts/**/*.sh' | each { |i| shellcheck $i } | str join "\n"
    ruff check '{{ probe }}'
    pyright '{{ root }}';

test:
    cd '{{ root }}'; cargo test

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

rebuild *args:
    nixos-rebuild switch --flake $"{{ root }}#pidgeon-(open --raw /etc/id)-aarch64-linux" {{ args }}

raspberryPi4 *args:
    {{ root }}/scripts/flake/raspberryPi4.nu {{ args }}
