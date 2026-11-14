"""Copies every shared fixture file into the mod's source, because a plugin test cannot read a file outside the plugin's folder.

Run from the repo root: `python3 scripts/gen-shared-fixtures.py` writes `plugin/hooks/book/shared.ts`;
`python3 scripts/gen-shared-fixtures.py --check` writes nothing and exits 1 when that file differs from what it would write.
"""

import json
import pathlib
import sys

SOURCE = pathlib.Path("fixtures/shared")
TARGET = pathlib.Path("plugin/hooks/book/shared.ts")

HEAD = """// A copy of every file below fixtures/shared, made by scripts/gen-shared-fixtures.py, because a plugin test cannot read a file outside the plugin's folder.
// Do not edit it by hand: change the fixture, then run the script again. Each value is the file's text exactly.

export const SHARED_FILES: { readonly [path: string]: string } = {
"""

TAIL = """}

export class MissingFixtureError extends Error {
  readonly path: string

  constructor(path: string) {
    super(`no copy of the shared fixture ${path}; add the file below fixtures/shared and run scripts/gen-shared-fixtures.py`)
    this.name = 'MissingFixtureError'
    this.path = path
  }
}

export function sharedText(path: string): string {
  const text = SHARED_FILES[path]
  if (text === undefined) {
    throw new MissingFixtureError(path)
  }
  return text
}

export function sharedJson(path: string): unknown {
  return JSON.parse(sharedText(path))
}
"""


def render() -> str:
    files = sorted(path for path in SOURCE.rglob("*.json") if path.is_file())
    if not files:
        raise SystemExit(f"gen-shared-fixtures: no JSON file below {SOURCE}; run it from the repo root")
    lines = [f"  {json.dumps(path.as_posix())}: {json.dumps(path.read_text(encoding='utf-8'), ensure_ascii=False)}," for path in files]
    return HEAD + "\n".join(lines) + "\n" + TAIL


def main(argv: list[str]) -> int:
    text = render()
    if argv == ["--check"]:
        current = TARGET.read_text(encoding="utf-8") if TARGET.is_file() else None
        if current != text:
            print(f"gen-shared-fixtures --check: {TARGET} differs from fixtures/shared; run python3 scripts/gen-shared-fixtures.py", file=sys.stderr)
            return 1
        return 0
    if argv:
        raise SystemExit(f"gen-shared-fixtures: unknown arguments {argv}; use no argument, or --check")
    TARGET.parent.mkdir(parents=True, exist_ok=True)
    TARGET.write_text(text, encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
