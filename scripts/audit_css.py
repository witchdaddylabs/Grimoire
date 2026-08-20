#!/usr/bin/env python3
"""Audit CSS rules against JSX/TSX class usage in Grimoire."""

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CSS_PATH = REPO_ROOT / "src" / "styles" / "global.css"

def get_defined_classes(css_text: str) -> set[str]:
    """Extract all CSS class selectors from global.css."""
    return set(re.findall(r"\.([a-zA-Z_-][a-zA-Z0-9_-]*)", css_text))

def extract_class_strings_from_expr(expr: str) -> list[str]:
    """Extract string literals that represent class names from a JSX expression."""
    strings_to_inspect: list[str] = []

    # Handle ternary: condition ? branch1 : branch2
    # String literals in condition (e.g. theme === "dark") are not class names.
    if "?" in expr and ":" in expr:
        # Split on top-level ? (simplistic but handles JSX ternaries well)
        _, branches = expr.split("?", 1)
        for m in re.finditer(r'(["\'`])((?:\\.|(?!\1).)*)\1', branches, re.DOTALL):
            strings_to_inspect.append(m.group(2))
    else:
        # Array literals, template literals, or direct strings
        for m in re.finditer(r'(["\'`])((?:\\.|(?!\1).)*)\1', expr, re.DOTALL):
            strings_to_inspect.append(m.group(2))

    results: list[str] = []
    for s in strings_to_inspect:
        # Remove template interpolations ${...}
        cleaned = re.sub(r"\$\{[^}]*\}", " ", s)
        for token in cleaned.split():
            token = token.strip()
            if token and not token.startswith("$") and re.match(r"^[a-zA-Z_-][a-zA-Z0-9_-]*$", token):
                results.append(token)
    return results

def get_used_classes(src_dir: Path) -> tuple[set[str], dict[str, set[str]]]:
    """Extract all class names used across TS/TSX source files."""
    used_classes: set[str] = set()
    usage_sources: dict[str, set[str]] = {}

    def add_class(cls_name: str, file_name: str) -> None:
        cls_name = cls_name.strip()
        if cls_name and not cls_name.startswith("$") and re.match(r"^[a-zA-Z_-][a-zA-Z0-9_-]*$", cls_name):
            used_classes.add(cls_name)
            usage_sources.setdefault(cls_name, set()).add(file_name)

    for p in src_dir.glob("**/*"):
        if p.suffix not in (".ts", ".tsx"):
            continue
        content = p.read_text(encoding="utf-8")
        file_name = p.name

        # 1. className="..."
        for m in re.finditer(r'className="([^"]*)"', content):
            for c in m.group(1).split():
                add_class(c, file_name)

        # 2. className={...}
        for m in re.finditer(r'className=\{([^{}]*(?:\{[^{}]*\}[^{}]*)*)\}', content, re.DOTALL):
            expr = m.group(1)
            for c in extract_class_strings_from_expr(expr):
                add_class(c, file_name)

        # 3. Dynamic helper functions and test assertions
        for m in re.finditer(r'toHaveClass\((["\'`])([^"\'`]+)\1\)', content):
            add_class(m.group(2), file_name)

    return used_classes, usage_sources

def get_orphaned_classes(defined_classes: set[str], src_dir: Path) -> list[str]:
    """Find CSS classes defined in stylesheet that are not referenced anywhere in src."""
    all_src = ""
    for p in src_dir.glob("**/*"):
        if p.suffix in (".ts", ".tsx", ".html"):
            all_src += p.read_text(encoding="utf-8") + "\n"

    orphaned = []
    for cls in sorted(defined_classes):
        if not re.search(r"\b" + re.escape(cls) + r"\b", all_src):
            orphaned.append(cls)
    return orphaned

def main() -> int:
    if not CSS_PATH.exists():
        print(f"Error: CSS file not found at {CSS_PATH}", file=sys.stderr)
        return 1

    src_dir = REPO_ROOT / "src"
    css_text = CSS_PATH.read_text(encoding="utf-8")

    defined = get_defined_classes(css_text)
    used, sources = get_used_classes(src_dir)

    missing = sorted([c for c in used if c not in defined])
    orphaned = get_orphaned_classes(defined, src_dir)

    print("=== 1. USED IN JSX/TS BUT NOT DEFINED IN CSS ===")
    if missing:
        for c in missing:
            srcs = ", ".join(sorted(sources.get(c, [])))
            print(f"  .{c:30} in {srcs}")
    else:
        print("  (None — all used classes have matching CSS rules)")

    print(f"\n=== 2. DEFINED IN CSS BUT NOT FOUND ANYWHERE IN SRC ({len(orphaned)} rules) ===")
    for o in orphaned:
        print(f"  .{o}")

    return 0 if not missing else 1

if __name__ == "__main__":
    sys.exit(main())
