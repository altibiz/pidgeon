root_path := justfile_directory()
scripts_path := join(root_path, 'scripts')
cli_path := join(root_path, 'src/cli')
probe_path := join(root_path, 'src/probe')
cli_config_path := join(cli_path, 'config.toml')


format:
	cd "{{root_path}}" && cargo fmt --all
	yapf --recursive --in-place --parallel "{{root_path}}"
	prettier --write "{{root_path}}"
	shfmt --write "{{root_path}}"

lint:
	cd "{{root_path}}" && cargo clippy
	ruff check "{{root_path}}"
	shellcheck "{{scripts_path}}"/*
	prettier --check "{{root_path}}"

build:
	cd "{{root_path}}" && cargo build --release

run *args:
	cd "{{cli_path}}" && cargo run -- --config "{{cli_config_path}}" {{args}}

probe *args:
  cd "{{probe_path}}" && python ./probe/main.py {{args}}
