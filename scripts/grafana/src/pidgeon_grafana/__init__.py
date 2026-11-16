import json
import sys
from pathlib import Path

from pidgeon_grafana.args import Args
from pidgeon_grafana.dashboards import DASHBOARDS
from pidgeon_grafana.shared import Json

def render(board: Json) -> str:
  return json.dumps(board, indent=2, ensure_ascii=False) + "\n"

def is_current(path: Path, content: str) -> bool:
  return path.exists() and path.read_text(encoding="utf-8") == content

def main():
  args = Args()
  out = args.out()
  files = {f"{board['uid']}.json": render(board) for board in DASHBOARDS}

  stale = [
    out / name for name, content in files.items()
    if not is_current(out / name, content)
  ]
  unknown = [
    path for path in sorted(out.glob("*.json")) if path.name not in files
  ]

  if args.check():
    for path in stale:
      print(f"out of date: {path}", file=sys.stderr)
    for path in unknown:
      print(f"not generated: {path}", file=sys.stderr)
    if stale or unknown:
      print("run `just grafana` to regenerate", file=sys.stderr)
      sys.exit(1)
    return

  out.mkdir(parents=True, exist_ok=True)
  for path in stale:
    path.write_text(files[path.name], encoding="utf-8", newline="\n")
    print(f"wrote {path}")
  for path in unknown:
    print(
      f"not generated, delete it if its dashboard was removed: {path}",
      file=sys.stderr,
    )


if __name__ == "__main__":
  main()
