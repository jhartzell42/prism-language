"""Extracts every ```prism fence of the Semantics, Type System, Templates and Conformance
draft books into one file each. Indented fragments are dedented. Usage:
    python3 -I extract.py <prism-notes>/claude/prism <out-dir>"""
import os, sys
root, out = sys.argv[1], sys.argv[2]
os.makedirs(out, exist_ok=True)
books = {"01-semantics": "sem", "02-type-system": "ts", "03-templates": "tpl", "04-conformance": "conf"}
n = 0
for b, short in books.items():
    d = os.path.join(root, b, "book")
    for f in sorted(os.listdir(d)):
        if not f.endswith(".md") or not f.startswith("ch-"):
            continue
        ch = f.split("-")[1]
        lines = open(os.path.join(d, f), encoding="utf-8").read().split("\n")
        i = 0; k = 0
        while i < len(lines):
            if lines[i].startswith("```prism"):
                j = i + 1
                while not lines[j].startswith("```"):
                    j += 1
                body = lines[i + 1:j]
                ind = min((len(l) - len(l.lstrip(" ")) for l in body if l.strip()), default=0)
                body = [l[ind:] for l in body]
                k += 1
                name = f"{short}-ch{ch}-{k:02d}.prism"
                with open(os.path.join(out, name), "w", encoding="utf-8") as fo:
                    fo.write(f"# {b}/book/{f}, line {i + 2}" + (f" (dedented {ind})" if ind else "") + "\n")
                    fo.write("\n".join(body) + "\n")
                n += 1
                i = j
            i += 1
print(n)
