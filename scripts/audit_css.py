import re, glob, os

css_path = "/Users/habibi/hermes/apps-codebases/grimoire/src/styles/global.css"
with open(css_path) as f:
    css_content = f.read()

# Only match class selectors in CSS
# We match selectors starting with a dot followed by standard CSS identifier chars
raw_css_classes = set(re.findall(r"\.([a-zA-Z_-][a-zA-Z0-9_-]*)", css_content))

src_files = glob.glob("/Users/habibi/hermes/apps-codebases/grimoire/src/**/*.tsx", recursive=True) + \
            glob.glob("/Users/habibi/hermes/apps-codebases/grimoire/src/**/*.ts", recursive=True)

used_classes = set()
used_details = {}

for p in src_files:
    with open(p) as f:
        content = f.read()
    
    # 1. className="..."
    for m in re.findall(r'className="([^"]*)"', content):
        for c in m.split():
            used_classes.add(c)
            used_details.setdefault(c, set()).add(f"{os.path.basename(p)}")
            
    # 2. className={`...`}
    for m in re.findall(r'className=\{`([^`]*)`\}', content):
        cleaned = re.sub(r"\$\{[^}]*\}", " ", m)
        for c in cleaned.split():
            used_classes.add(c)
            used_details.setdefault(c, set()).add(f"{os.path.basename(p)}")

    # 3. test assertions like toHaveClass("...")
    for m in re.findall(r'toHaveClass\("([^"]+)"\)', content):
        used_classes.add(m)
        used_details.setdefault(m, set()).add(f"{os.path.basename(p)}")

print("=== 1. USED IN JSX/TS BUT NOT DEFINED IN CSS ===")
missing = sorted([c for c in used_classes if c not in raw_css_classes])
for m in missing:
    print(f"  .{m:30} in {used_details[m]}")

all_src_code = ""
for p in src_files:
    with open(p) as f:
        all_src_code += f.read() + "\n"

print("\n=== 2. DEFINED IN CSS BUT NOT FOUND ANYWHERE IN SRC ===")
orphaned = []
for c in sorted(raw_css_classes):
    if not re.search(r'\b' + re.escape(c) + r'\b', all_src_code):
        orphaned.append(c)

for o in orphaned:
    print(f"  .{o}")
print(f"Total orphaned rules: {len(orphaned)}")
