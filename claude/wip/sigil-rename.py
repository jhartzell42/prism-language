#!/usr/bin/env python3
"""Move field sigils from a field's type to its name, and mark named arguments at sigiled
fields: `label: $Text` -> `$label: Text`, `Ui.Button(label=$x)` -> `Ui.Button($label=$x)`.

usage: sigil-rename.py [--write] DIR_OR_FILE...
A file holding the line `# sigil-rename: skip` is left alone (a test of the old form).
Run it from the repository root: the prelude's declarations (./prelude) always count.
Each program's declarations (its directory tree up to its root file) and the prelude's mark
its own constructions only. Idempotent: a declaration already written `$label: Text` is read for its sigils and kept, and
an argument already written `$label=` is kept.
World patterns keep plain names (they read sampled levels): `expect` lines and every line of a
`test` block."""
import os, re, sys

SIG = "$!?~"
decl_re = re.compile(r'^(type\s+([A-Z][A-Za-z0-9]*)(\s+[a-z][a-z0-9]*)*\s*=\s*record:\s*(#.*)?$)|^(tag\s+([A-Z][A-Za-z0-9.]*)\s.*:\s*(#.*)?$)')
field_re = re.compile(r'^(\s+)([a-z][A-Za-z0-9]*): ([' + re.escape(SIG) + r']+)(\S.*)$')
plainfield_re = re.compile(r'^(\s+)([$!?~]*)([a-z][A-Za-z0-9]*): ')

def files(paths):
    for p in paths:
        if os.path.isfile(p):
            yield p
        else:
            for d, ds, fs in os.walk(p):
                ds[:] = [x for x in ds if x != 'target']
                for f in fs:
                    if f.endswith('.prism'):
                        yield os.path.join(d, f)

def collect(fs):
    """owner (bare and qualified) -> {field: sigils}; also rewrites declarations."""
    table = {}
    newtext = {}
    for f in fs:
        lines = open(f).read().split('\n')
        out = []
        owner = None
        indent = None
        for line in lines:
            m = decl_re.match(line)
            if m:
                owner = m.group(2) or m.group(6)
                indent = None
                table.setdefault(owner, {})
                out.append(line)
                continue
            if owner is not None:
                stripped = line.strip()
                if stripped == '' or (not line.startswith((' ', '\t'))):
                    owner = None if not stripped.startswith('#') or not line.startswith(' ') else owner
                    if owner is None:
                        out.append(line)
                        continue
                if stripped.startswith('#'):
                    out.append(line)
                    continue
                fm = field_re.match(line)
                pm = plainfield_re.match(line)
                if fm:
                    ind, name, sigs, rest = fm.groups()
                    table[owner][name] = sigs
                    out.append(f"{ind}{sigs}{name}: {rest}")
                    continue
                elif pm:
                    # already renamed (`$label: Text`): its sigils still mark its arguments
                    if pm.group(2):
                        table[owner][pm.group(3)] = pm.group(2)
                    out.append(line)
                    continue
                else:
                    owner = None
            out.append(line)
        newtext[f] = '\n'.join(out)
    return table, newtext

ctor_re = re.compile(r'([A-Z][A-Za-z0-9]*(?:\.[A-Z][A-Za-z0-9]*)*)\(')

def scan_args(text, i, fields, edits):
    """text[i] is just after '(' of a constructor; mark named args at sigiled fields."""
    depth = 0
    at_start = True
    n = len(text)
    while i < n:
        c = text[i]
        if at_start and depth == 0:
            m = re.match(r'\s*([a-z][A-Za-z0-9]*)=(?!=)', text[i:])
            if m:
                name = m.group(1)
                if name in fields:
                    edits.append((i + m.start(1), fields[name]))
            at_start = False
        if c == '"':
            i = skip_string(text, i)
            continue
        if c in '([{':
            depth += 1
        elif c in ')]}':
            if depth == 0:
                return i
            depth -= 1
        elif c == ',' and depth == 0:
            at_start = True
        i += 1
    return i

def skip_string(text, i):
    """text[i] == '"'; return index after the closing quote, through `{...}` holes."""
    i += 1
    n = len(text)
    while i < n:
        c = text[i]
        if c == '\\':
            i += 2
            continue
        if c == '"':
            return i + 1
        if c == '{':
            i = skip_hole(text, i + 1)
            continue
        i += 1
    return i

def skip_hole(text, i):
    depth = 0
    n = len(text)
    while i < n:
        c = text[i]
        if c == '"':
            i = skip_string(text, i)
            continue
        if c in '([{':
            depth += 1
        elif c in ')]}':
            if depth == 0 and c == '}':
                return i + 1
            depth -= 1
        i += 1
    return i

def test_block_spans(text):
    """The spans of `test NAME, @w:` blocks: their steps are World patterns, which read
    sampled levels by plain names (`click Ui.Button(label="Go")`)."""
    spans = []
    lines = text.split('\n')
    pos = 0
    start = None
    for line in lines:
        if start is not None and line.strip() != '' and not line.startswith((' ', '\t')):
            spans.append((start, pos))
            start = None
        if re.match(r'^test\s', line):
            start = pos
        pos += len(line) + 1
    if start is not None:
        spans.append((start, pos))
    return spans

def mark_args(text, table, conflicts):
    edits = []
    tests = test_block_spans(text)
    for m in ctor_re.finditer(text):
        start = m.start()
        if any(a <= start < b for a, b in tests):
            continue
        line_start = text.rfind('\n', 0, start) + 1
        line_end = text.find('\n', start)
        line = text[line_start:line_end if line_end >= 0 else len(text)]
        if re.search(r'\bexpect\b', line) or line.lstrip().startswith('#'):
            continue
        if start > 0 and (text[start - 1].isalnum() or text[start - 1] in '_'):
            continue
        name = m.group(1)
        key = name if name in table else name.split('.')[-1]
        if key not in table:
            continue
        if key in conflicts:
            print(f"conflict: {key} used at offset {start}", file=sys.stderr)
        fields = {k: v for k, v in table[key].items() if v}
        if not fields:
            continue
        scan_args(text, m.end(), fields, edits)
    for pos, sigs in sorted(edits, reverse=True):
        text = text[:pos] + sigs + text[pos:]
    return text, len(edits)

SKIP = '# sigil-rename: skip'
ROOTS = ('app.prism', 'lib.prism', 'mod.prism')

def program_root(f):
    """The program a file belongs to: the outermost directory above it holding a root file,
    up to the first path component; a lone file with none is its own program."""
    d = os.path.dirname(os.path.abspath(f))
    best = None
    while True:
        if any(os.path.exists(os.path.join(d, r)) for r in ROOTS):
            best = d
        parent = os.path.dirname(d)
        if parent == d or os.path.basename(parent) in ('tests', 'examples', 'bench', '') or d == os.getcwd():
            break
        d = parent
    return best or os.path.abspath(f)

def main():
    write = '--write' in sys.argv
    paths = [a for a in sys.argv[1:] if a != '--write']
    fs = [f for f in files(paths) if SKIP not in open(f).read()]
    prelude = [f for f in fs if os.path.abspath(f).startswith(os.path.abspath('prelude') + os.sep)]
    # the prelude's declarations reach every program, even when it is not being rewritten
    preludeRead = sorted(set(prelude) | set(files(['prelude'])) if os.path.isdir('prelude') else set(prelude))
    groups = {}
    for f in fs:
        if f not in prelude:
            groups.setdefault(program_root(f), []).append(f)
    # the prelude's declarations reach every program; a program's own reach only it
    ptable, pdecl = collect(preludeRead)
    out = {}
    for f in prelude:
        out[f] = (pdecl[f], ptable)
    for root, members in groups.items():
        table, declared = collect(members)
        merged = {k: dict(v) for k, v in ptable.items()}
        for k, v in table.items():
            merged.setdefault(k, {}).update(v)
        for f in members:
            out[f] = (declared[f], merged)
    total = 0
    conflicts = set()
    for f, (text0, table) in out.items():
        text, n = mark_args(text0, table, conflicts)
        if text != open(f).read():
            total += n
            if write:
                open(f, 'w').write(text)
            else:
                print(f"{f}: {n} args")
    print(f"args marked: {total}", file=sys.stderr)

def unused_main():
    write = '--write' in sys.argv
    paths = [a for a in sys.argv[1:] if a != '--write']
    fs = list(files(paths))
    table, declared = collect(fs)
    # a bare owner name declared twice with different sigiled fields is a conflict
    conflicts = set()
    total = 0
    for f in fs:
        text, n = mark_args(declared[f], table, conflicts)
        orig = open(f).read()
        if text != orig:
            total += n
            if write:
                open(f, 'w').write(text)
            else:
                print(f"{f}: {n} args")
    sig = {o: {k: v for k, v in fl.items() if v} for o, fl in table.items()}
    print(f"owners with sigiled fields: {sum(1 for o in sig if sig[o])}; args marked: {total}", file=sys.stderr)

main()
