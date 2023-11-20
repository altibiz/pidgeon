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

probe *args:
  cd "{{join(root, 'src/probe')}}" && python ./probe/main.py {{args}}
