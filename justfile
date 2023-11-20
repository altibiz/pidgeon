root := justfile_directory()
scripts := join(root, 'scripts')

format:
	cd "{{root}}" && cargo fmt --all
	yapf --recursive --in-place --parallel "{{root}}"
	prettier --write "{{root}}"
	shfmt --write "{{root}}"

lint:
	cd "{{root}}" && cargo clippy
	ruff check "{{root}}"
	shellcheck "{{scripts}}"/*
	prettier --check "{{root}}"

default_slave := '0'
probe ip device slave=default_slave:
  cd "{{join(root, 'src/probe')}}" && \
    python ./probe/main.py -i "{{ip}}" -d "{{device}}" -s "{{slave}}"
