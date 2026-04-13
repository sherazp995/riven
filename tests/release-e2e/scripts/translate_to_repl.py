#!/usr/bin/env python3
"""Translate a rivenc fixture (.rvn) into equivalent REPL input.

The rivenc harness runs full programs that each define a `def main
... end` entry point. The REPL evaluates top-level statements one at
a time. To share the same `.out` expectations with the REPL harness,
we strip the `def main` wrapper and hoist its body to the top level,
leaving every other top-level item (class / struct / enum / trait /
impl / const / type / fn) verbatim.

Reads from stdin, writes to stdout.
"""
from __future__ import annotations

import re
import sys


# Tokens that open a new multi-line scope (closed by `end`).
OPENERS = {
    "def", "while", "if", "for", "loop", "match", "do",
    "unsafe", "class", "struct", "enum", "trait", "impl", "module",
}


def first_keyword(line: str) -> str | None:
    m = re.match(r"\s*([a-zA-Z_][a-zA-Z0-9_]*)\b", line)
    return m.group(1) if m else None


def translate(src: str) -> str:
    lines = src.split("\n")
    out: list[str] = []
    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()
        # Spot `def main` at column 0 with no args or with `def main()`.
        if re.match(r"^def\s+main\s*(\(\s*\))?\s*$", stripped) and not line.startswith(" "):
            # Consume the body, dedent by the first-line indent,
            # and skip the matching `end`.
            depth = 1
            body: list[str] = []
            i += 1
            while i < len(lines):
                b = lines[i]
                bs = b.strip()
                kw = first_keyword(b)
                if bs == "end":
                    depth -= 1
                    if depth == 0:
                        i += 1
                        break
                    body.append(b)
                elif kw in OPENERS:
                    # Inline one-liners like `def foo(x) { ... }` don't
                    # open a new end-closed scope; detect `{ ... }` same line.
                    if "{" in bs and bs.endswith("}"):
                        body.append(b)
                    else:
                        depth += 1
                        body.append(b)
                else:
                    body.append(b)
                i += 1
            # Dedent by the smallest nonzero leading-space count.
            nonblank = [ln for ln in body if ln.strip()]
            if nonblank:
                min_indent = min(len(ln) - len(ln.lstrip(" ")) for ln in nonblank)
                body = [ln[min_indent:] if len(ln) >= min_indent else ln for ln in body]
            out.extend(body)
        else:
            out.append(line)
            i += 1
    return "\n".join(out)


if __name__ == "__main__":
    sys.stdout.write(translate(sys.stdin.read()))
