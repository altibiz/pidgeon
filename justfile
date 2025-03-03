set windows-shell := ["nu.exe", "-c"]
set shell := ["nu", "-c"]

root := absolute_path('')
scripts := absolute_path('scripts')
cli := absolute_path('src/cli')
probe := absolute_path('src/probe')
assets := absolute_path('assets')
config := absolute_path('assets/config.toml')
artifacts := absolute_path('artifacts')
target := absolute_path('target')
docs := absolute_path('docs')
isready := absolute_path('scripts/isready.nu')
host-script := absolute_path('scripts/host.nu')

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
          | get id) \
        docker exec $timescale_container_id pg_isready --host localhost \
        break \
      } catch { \
        sleep 1sec \
        continue \
      } \
    }
    cd '{{ cli }}'; cargo sqlx migrate run

lfs:
    dvc add {{ assets }}/*.csv
    dvc add {{ assets }}/*.sql
    dvc push

run *args:
    cd '{{ cli }}'; cargo run -- --config '{{ config }}' {{ args }}

probe *args:
    cd '{{ probe }}'; \
      $env.PIDGEON_PROBE_ENV = 'development'; \
      python -m probe.main {{ args }}

format:
    cd '{{ root }}'; just --unstable --fmt
    prettier --write '{{ root }}'
    cd '{{ root }}'; cargo fmt --all
    yapf --recursive --in-place --parallel '{{ probe }}'
    shfmt --write '{{ root }}'
    nixpkgs-fmt '{{ root }}'

lint:
    cd '{{ root }}'; just --unstable --fmt --check
    nixpkgs-fmt '{{ root }}' --check
    prettier --check '{{ root }}'
    cspell lint '{{ root }}' --no-progress
    cd '{{ root }}'; cargo clippy -- -D warnings
    glob '{{ scripts }}/*.sh' | each { |i| shellcheck $i } | str join "\n"
    ruff check '{{ probe }}'
    pyright '{{ root }}';

test:
    cd '{{ root }}'; cargo test

upgrade:
    nix flake update
    cargo upgrade

docs:
    rm -rf '{{ artifacts }}'
    mkdir '{{ artifacts }}'
    cd '{{ root }}'; cargo doc --no-deps
    cd '{{ docs }}/en'; mdbook build
    cd '{{ docs }}/hr'; mdbook build
    mv '{{ target }}/doc' '{{ artifacts }}/code'
    mv '{{ docs }}/en/book' '{{ artifacts }}/en'
    mv '{{ docs }}/hr/book' '{{ artifacts }}/hr'
    cp '{{ docs }}/index.html' '{{ artifacts }}'

rebuild *args:
    nixos-rebuild switch --flake $"{{ root }}#pidgeon-(open --raw /etc/id)-aarch64-linux" {{ args }}

raspberryPi4 *args:
    {{ root }}/scripts/flake/raspberryPi4.nu {{ args }}
