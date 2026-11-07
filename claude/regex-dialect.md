# The regular-expression dialect, as the runtimes check it

Written by Claude on the claude-self-hosting branch; subject to change.

The books fix the dialect (Semantics Appendix B, "Regular expressions" and "The dialect";
chapter 2, "Regular expressions"; rulings round 5, RX to RX-DOC; Type System chapter 2).
This file is the procedure that turns those pages into one accept/refuse decision, one
error position and one message, so that the Rust runtime (`runtime/rust/src/regex.rs`,
`regex_dialect.rs`), the C++ runtime (`runtime/cpp/src/regex.cpp`) and prismc's
checker (`prismc/check/regex/`, in Prism) refuse the same patterns the same way. The
shared corpus `tests/regex/corpus.txt` pins every row below; both runtimes' tests read
it, and prismc's test module is written from it.

Where the books are silent or the two libraries could not be made to agree, the
construct is refused and the call is a doubt with a two-letter code (the last section).

## Inputs and diagnostics

- **Three texts:** the *pattern* (as written between a literal's delimiters: the plain
  form's `\/` reaches the runtime as `\/`, which the dialect reads as a `/`; or the text
  given to `?Regex`), the *flags* (the letters after the last slash; empty for
  `?Regex`), and for a `RegexReplacement` the *replacement* (as written between its
  slashes). All are UTF-8 (a `Text` cannot be anything else).
- **A diagnostic** is `(part, at, message)`: `part` is `flags`, `pattern` or
  `replacement`; `at` a byte offset into that part; `message` one of the fixed texts in
  the tables below, word for word (no part of a message depends on the input). The
  corpus writes one as `pattern@3 a repetition of a repetition: write (?:a*)* to say it`.
- **Order:** the flags are checked first, then the pattern, then the replacement; within
  each, the first error met reading left to right wins, with the two whole-pattern
  checks (nesting, size) after the pattern has parsed.
- **Results:** `Regex` construction answers the regex or the diagnostic; `?Regex` is
  `nil` for any diagnostic; a literal that fails is a prismc diagnostic before tick 0
  (the runtime's literal constructor treats it as a fatal error, never reached once
  prismc checks).

## Flags

| Input | Result |
| --- | --- |
| each of `i`, `m`, `s`, at most once, in any order | accepted |
| any other character | `flags@k an unknown flag: the flags are i, m and s` (k: its byte offset) |
| a flag a second time | `flags@k a flag written twice` |

## The pattern, read left to right

A pattern is a sequence of code points; positions are byte offsets. The parse keeps
the *flag state* (`i`, `m`, `s`), which starts as the flags above; a flag group changes
it to the end of the enclosing group (across `|`, as in both libraries), a scoped flag
group only inside itself.

### Outside a class

| At | Reading | Diagnostic otherwise (at the character named) |
| --- | --- | --- |
| a code point other than `\ . + * ? ( ) \| [ ] { } ^ $` | itself, spaces and raw newlines included | |
| `.` | any code point but `\n`; any under `s` | |
| `^` | `\A`; under `m`, also just after each `\n` | |
| `$` | `\z`; under `m`, also just before each `\n` | |
| `\|` | alternation; branches may be empty (`a\|`, `\|`, `()`) | |
| `]` | | `a ] outside a class: a literal one is \]` |
| `}` | | `a } outside a count: a literal one is \}` |
| `*` `+` `?` `{` | a repetition of the piece before (below) | |
| `(` | a group (below) | |
| `[` | a class (below) | |
| `\` | an escape (below) | |

### Escapes (outside a class); every diagnostic is at the `\`

| Escape | Reading / diagnostic |
| --- | --- |
| `\` at the end | `a \ at the end of the pattern` |
| `\` + ASCII punctuation but `<` `>`: ``! " # $ % & ' ( ) * + , - . / : ; = ? @ [ \ ] ^ _ ` { \| } ~`` | that character |
| `\t` `\n` `\r` `\f` `\v` `\a` | U+0009, U+000A, U+000D, U+000C, U+000B, U+0007 |
| `\xhh` (exactly two hex digits, either case) | that code point |
| `\x{h...}` | one to six hex digits: that code point |
| `\x` followed by neither | `a \x escape needs two hex digits or braces` |
| `\x{` with no digits, a non-hex character before `}`, more than six digits, or no `}` | `a \x{...} escape needs one to six hex digits` |
| `\x` naming a surrogate (D800-DFFF) or past 10FFFF | `a \x escape names no character: a surrogate or past U+10FFFF` |
| `\d` `\w` `\s` | `[0-9]`, `[0-9A-Za-z_]`, `[\t\n\v\f\r ]` (ASCII, RX-DL) |
| `\D` `\W` `\S` | their negations: `[^0-9]`, `[^0-9A-Za-z_]`, `[^\t\n\v\f\r ]` |
| `\pX` `\p{Name}` `\PX` `\P{Name}` | a property (below), `\P` its negation |
| `\A` `\z` | the text's front and back, always |
| `\b` `\B` | an ASCII word boundary (between a `\w` and a non-`\w`, or a `\w` and an end of the text); its negation |
| `\b{` | `\b{...} is Rust's only: write \b` |
| `\<` `\>` | `\< and \> are Rust's only: write \b` |
| `\1` to `\9` | `a backreference: refused so that every match is linear in its text` |
| `\0` | `an octal escape is RE2's only: write \x` |
| `\Q` `\E` | `\Q...\E is RE2's only: escape each character` |
| `\C` | `\C, one byte, is RE2's only` |
| anything else (another letter, a space, a non-ASCII code point) | `an unknown escape` |

### Properties; every diagnostic is at the `\`

| Form | Reading / diagnostic |
| --- | --- |
| `\p` or `\P` at the end, or followed by a character that is not `{` or an ASCII letter | `a \p escape needs a property name` |
| `\p{` with no `}` | `a \p{ that is never closed` |
| `\p{^...}` | `a negated property is written \P{...}` |
| a name in the table below, exactly (case and underscores as written) | accepted |
| `C`, `Cn`, `Cs`, `LC` | `the property is refused: RE2 and Rust disagree on C, Cn, Cs and LC` (doubt PC) |
| any other name (long names, `gc=`, `sc=`, binary properties, script extensions, lowercase spellings, spaces) | `an unknown property: general categories by short name, scripts and Any` |

- **General categories (34):** `Cc Cf Co L Ll Lm Lo Lt Lu M Mc Me Mn N Nd Nl No P Pc Pd Pe
  Pf Pi Po Ps S Sc Sk Sm So Z Zl Zp Zs`; one letter may be written bare (`\pL`).
- **Any.**
- **Scripts (163, Unicode 15.0's, the names both libraries accept):** Adlam Ahom
  Anatolian_Hieroglyphs Arabic Armenian Avestan Balinese Bamum Bassa_Vah Batak Bengali
  Bhaiksuki Bopomofo Brahmi Braille Buginese Buhid Canadian_Aboriginal Carian
  Caucasian_Albanian Chakma Cham Cherokee Chorasmian Common Coptic Cuneiform Cypriot
  Cypro_Minoan Cyrillic Deseret Devanagari Dives_Akuru Dogra Duployan
  Egyptian_Hieroglyphs Elbasan Elymaic Ethiopic Georgian Glagolitic Gothic Grantha Greek
  Gujarati Gunjala_Gondi Gurmukhi Han Hangul Hanifi_Rohingya Hanunoo Hatran Hebrew
  Hiragana Imperial_Aramaic Inherited Inscriptional_Pahlavi Inscriptional_Parthian
  Javanese Kaithi Kannada Katakana Kawi Kayah_Li Kharoshthi Khitan_Small_Script Khmer
  Khojki Khudawadi Lao Latin Lepcha Limbu Linear_A Linear_B Lisu Lycian Lydian Mahajani
  Makasar Malayalam Mandaic Manichaean Marchen Masaram_Gondi Medefaidrin Meetei_Mayek
  Mende_Kikakui Meroitic_Cursive Meroitic_Hieroglyphs Miao Modi Mongolian Mro Multani
  Myanmar Nabataean Nag_Mundari Nandinagari New_Tai_Lue Newa Nko Nushu
  Nyiakeng_Puachue_Hmong Ogham Ol_Chiki Old_Hungarian Old_Italic Old_North_Arabian
  Old_Permic Old_Persian Old_Sogdian Old_South_Arabian Old_Turkic Old_Uyghur Oriya Osage
  Osmanya Pahawh_Hmong Palmyrene Pau_Cin_Hau Phags_Pa Phoenician Psalter_Pahlavi Rejang
  Runic Samaritan Saurashtra Sharada Shavian Siddham SignWriting Sinhala Sogdian
  Sora_Sompeng Soyombo Sundanese Syloti_Nagri Syriac Tagalog Tagbanwa Tai_Le Tai_Tham
  Tai_Viet Takri Tamil Tangsa Tangut Telugu Thaana Thai Tibetan Tifinagh Tirhuta Toto
  Ugaritic Vai Vithkuqi Wancho Warang_Citi Yezidi Yi Zanabazar_Square. Unicode 16.0's
  seven (Garay, Gurung_Khema, Kirat_Rai, Ol_Onal, Sunuwar, Todhri, Tulu_Tigalari) are
  unknown: RE2 lacks them (doubt UN).

### Classes `[...]`

After `[` an optional `^` negates. Then items until the `]` that closes, with these
rules, every diagnostic at the character named:

| At | Reading / diagnostic |
| --- | --- |
| the end of the pattern before the closing `]` | `a class opened here is never closed` (at the `[`) |
| `]` as the first item (`[]`, `[^]`, `[]a]`) | `a ] first in a class: write \]` |
| `]` otherwise | closes the class |
| `[:name:]` or `[:^name:]`, name one of `alnum alpha ascii blank cntrl digit graph lower print punct space upper word xdigit` | that ASCII class (POSIX's, as both libraries define them), `^` negating it |
| `[:` letters `:]` with another name | `an unknown ASCII class` (at the `[`) |
| any other `[` | `a [ inside a class: write \[ (nested classes are Rust's only)` |
| `&&`, `~~`, `--` (unescaped, anywhere, endpoints included) | `&&, ~~ and -- in a class are Rust's set operations: escape them` (at the first) |
| `-` as the first item (after the `^`, if any) or right before the closing `]` | itself |
| `-` elsewhere, not between two single-character endpoints | `a - in a class that is neither a range nor first or last: write \-` |
| `a-b`, both ends single characters (a code point or a character escape) | the code points from `a` to `b` |
| a range whose left end is greater | `a range whose ends are out of order` (at the left end) |
| a range whose right end is a class escape (`\d`, `\p{...}`, ...) | `a range end that is a class` (at its `\`) |
| `\` + ASCII punctuation but `<` `>`, `\t \n \r \f \v \a`, `\x...` | a character, with the diagnostics of the escapes table |
| `\d \w \s \D \W \S \p \P` | that set, with the diagnostics of the properties table |
| `\D \W \S \P...` or `[:^name:]` inside a class while `i` is on | `a negated class inside a class under i: write it outside the class` (doubt NI) |
| any other escape (`\b`, `\A`, a digit, `\<`, ...) | `an escape that a class cannot hold` |
| any other code point (`^` not first, `&`, `~`, `.`, `$`, ...) | itself |

The ASCII classes: `alnum` `[0-9A-Za-z]`, `alpha` `[A-Za-z]`, `ascii` `[\x00-\x7F]`,
`blank` `[\t ]`, `cntrl` `[\x00-\x1F\x7F]`, `digit` `[0-9]`, `graph` `[!-~]`, `lower`
`[a-z]`, `print` `[ -~]`, `punct` ``[!-/:-@\[-`{-~]``, `space` `[\t\n\v\f\r ]`, `upper`
`[A-Z]`, `word` `[0-9A-Za-z_]`, `xdigit` `[0-9A-Fa-f]`.

### Groups; every diagnostic at the `(` unless said

| Form | Reading / diagnostic |
| --- | --- |
| `(re)` | captured, numbered by its `(` from 1 |
| `(?:re)` | not captured |
| `(?<name>re)`, `(?P<name>re)` | captured, numbered, and named |
| `(?flags)` | sets the flag state to the end of the enclosing group; not a piece (nothing can repeat it) |
| `(?flags:re)` | `re` under those flags |
| `(?=` `(?!` `(?<=` `(?<!` | `a lookaround: refused so that every match is linear in its text` |
| `(?P=` `(?P>` | `a backreference: refused so that every match is linear in its text` |
| `(?` followed by anything else (`(?#`, `(?>`, `(?\|`, `(?P` other, a digit, a non-letter in the flags) | `an unknown group: the groups are (re), (?:re), (?<name>re), (?P<name>re) and the flag groups` |
| a name not `[a-z_][a-z0-9_]*` (a Prism value name, lowercase), empty included | `a group name is a lowercase value name, [a-z_][a-z0-9_]*` (at the name's first byte) |
| a name with no `>` | `a group name that is never closed` |
| a name used before in the pattern | `a group name used twice` |
| the end of the pattern inside a group | `a group opened here is never closed` (at the innermost open `(`) |
| `)` with no group open | `a ) that closes no group` (at the `)`) |
| a group whose opening (`(`, `(?:`, `(?<name>`, `(?flags:`) has been read while 100 are open | `groups and repetitions nested more than 100 deep` (at 0; the nesting check below, made early, after the opening's own diagnostics) |

Flags inside `(?` ... `)` or `(?` ... `:`: letters and one `-`, every diagnostic at the
character named but the empty group's:

| Flags | Diagnostic |
| --- | --- |
| `(?)` | `an empty flag group` (at the `(`) |
| a letter other than `i` `m` `s` (ASCII letters only; another character is an unknown group) | `an unknown flag: the flags are i, m and s` |
| a flag already named in this group, either side of `-` | `a flag written twice` |
| a second `-` | `a second - in a flag group` |
| `-` right before the `)` or `:` | `a - that turns off no flag` |

### Repetition; every diagnostic at the operator (the `*`, `+`, `?` or `{`)

| At | Reading / diagnostic |
| --- | --- |
| `{` not followed by digits and then `}`, `,}` or `,` digits `}` (`{`, `{,3}`, `{a}`, `{ 2}`) | `a brace that opens no count: a literal one is \{` |
| a count with a leading zero (`{01}`, `{1,02}`) | `a count with a leading zero` (doubt CZ) |
| a count above 1000 | `a count above 1000` |
| `{n,m}` with `n > m` | `a count whose minimum exceeds its maximum` |
| nothing before it (start of a branch, after `(` or `\|`, or after a flag group) | `a repetition with nothing to repeat` |
| after a repetition (`a**`, `a+*`, `a{2}{3}`, `a???`) | `a repetition of a repetition: write (?:a*)* to say it` |
| after `^`, `$`, `\A`, `\z`, `\b` or `\B` | `a repetition of an assertion` (doubt AR) |
| nested counts whose product passes 1000 | `nested counts multiplying past 1000` |
| otherwise | `*` `+` `?` `{n}` `{n,}` `{n,m}`, greedy; a `?` right after makes it lazy (and is not a repetition) |

The checks run in that order: a count's shape, zero, bound and order first, then what
it repeats, then the product. A count's *factor* is its bound (`m` of `{n,m}`, `n` of
`{n}` and `{n,}`), or 1 when that is 0; `*`, `+` and `?` have factor 1. The *product* of
a count is its factor times the largest product of a count inside what it repeats (1
if none); it is checked when the count is read. (This is RE2's budget, `1000` divided
down the tree, which refuses exactly the products above 1000.)

### The whole-pattern checks, after the parse

1. **Nesting:** the depth of a code point, `.`, class, escape or assertion is 0; of a
   group (any kind but a bare `(?flags)`) 1 more than the deepest thing in it; of a
   repetition 1 more than what it repeats. Over 100: `pattern@0 groups and repetitions
   nested more than 100 deep` (doubt DP).
2. **Size:** a code point, `.`, a class, an escape or an assertion counts 1; a group
   what it holds; an alternation or a sequence the sum; a repetition what it repeats
   times its factor. Over 10,000: `pattern@0 the pattern expands past 10,000 atoms`
   (doubt SZ).

## What a pattern means

- **Leftmost-first** (Appendix B, "Each search is leftmost-first"): the match that starts
  earliest, and among those the one the pattern prefers, alternatives left to right,
  greedy repetition as far as it goes, lazy as little. Both libraries' default; RE2's
  longest-match option is never set.
- **Case folding** under `i` is Unicode simple case folding (RX-F, RX-DL), as both
  libraries do it: `k` matches `K` and U+212A KELVIN SIGN, `s` matches U+017F, `ß`
  matches U+1E9E and not `ss`, `σ` matches `Σ` and `ς`. It applies to classes, escaped
  ones included: a class under `i` matches every code point equivalent under folding to
  one of its members, so `(?i)[a-z]` and `(?i)\w` match U+212A; a negated class, `\W`
  and `\P{...}` under `i` match what that folded set does not (doubt NI for negations
  inside a class).
- **`\b`** is ASCII: `é` is not a word character, so `\b` holds between `a` and `é`.
- **No empty match inside a character** (RX-DL): an empty match RE2 reports between the
  bytes of one character is skipped and the search goes on at the next character.
- **Context:** a search started past the front sees the whole text: `^` without `m` and
  `\A` match only at byte 0; `^` under `m` after a `\n`; `\b` looks at the character
  before the start. `?Match` at a cursor is anchored there with that context.
- **Iteration** (Appendix B, the iteration rule) for `Matches`, `Split` and `Replace`:
  search from the front; after a match, the next search starts where it ended; a match
  that is empty and starts where the previous match ended is not reported, and the
  search is made again from the next character boundary; the end of the text ends the
  loop. `Matches %/a*/, "baaa"` is `[0,0)` and `[1,4)`; `Split %//, "abc"` is
  `["", "a", "b", "c", ""]`; `Split %/,/, ""` is `[""]`.

## How each runtime spells a pattern to its library

Neither library sees the pattern as written: the runtime's parse emits a spelling its
library reads with exactly the meaning above, so that the libraries' differences (which
escapes they take, ASCII or Unicode `\d`, `\s` with or without `\v`, what `(?-u:...)`
means) never reach a match. Every literal code point and every range end is written
`\x{...}` unless it is an ASCII letter or digit; `(?i:...)` wraps each atom read under
`i`; the flags `m` and `s` are never passed, the anchors and `.` being spelled for them.

| Construct | Rust (`regex-automata`) | C++ (RE2) |
| --- | --- | --- |
| `.` / under `s` | `[^\x{a}]` / `(?s:.)` | the same |
| `^` / under `m` | `\A` / `(?m:^)` | the same |
| `$` / under `m` | `\z` / `(?m:$)` | the same |
| `\b`, `\B` | `(?-u:\b)`, `(?-u:\B)` | `\b`, `\B` |
| `\d \w \s` | `[0-9]`, `[0-9A-Za-z_]`, `[\x{9}-\x{d}\x{20}]` | the same |
| `\D \W \S` alone | `[^...]` of the above | the same |
| `\D \W \S`, `[:^name:]` in a class | the complement as explicit ranges | the same |
| `[:name:]` | its ASCII ranges | the same |
| `\p{...}`, `\P{...}` | as written, in braces | the same |
| `(re)`, `(?<n>re)` | `(re)`: names are the runtime's own table | the same |
| `(?:re)`, `(?flags:re)` | `(?:re)` | the same |
| `(?flags)` | nothing (the parse's flag state) | the same |
| repetitions | as written | the same |

The libraries' limits are raised to fit the bounds above: Rust's syntax nesting limit
to 1000 and its NFA size limit removed; RE2's `max_mem` to 1 GiB.

## Replacements (chapter 2, "The replacement"; RX-E)

Read left to right; each diagnostic at the `$` or `\` that begins the bad form:

| Form | Reading / diagnostic |
| --- | --- |
| a code point other than `$` and `\` | itself (raw spaces, `"` and newlines included) |
| `$$` | `$` |
| `$` and digits (all that follow) | that group (`$0` the whole match, `$1a` group 1 then `a`) |
| `${` digits `}` | that group |
| `${` name `}`, name `[a-z_][a-z0-9_]*` | that named group |
| `$` digits with a leading zero (`$01`, `${01}`) | `a group number with a leading zero` (doubt RZ) |
| `${` anything else, or no `}` | `a ${...} that is neither a group number nor a group name` |
| `$` followed by anything else, or at the end | `a $ that is not $n, ${n}, ${name} or $$` |
| a group number past the pattern's | `the pattern has no group of that number` |
| a name the pattern does not have | `the pattern has no group of that name` |
| `\\` `\n` `\t` `\/` | `\`, U+000A, U+0009, `/` |
| `\u{h...}`, one to six hex digits, a character | that character |
| `\u` otherwise | `a \u{...} escape needs one to six hex digits naming a character` |
| `\` followed by anything else, or at the end | `an unknown escape in a replacement: the escapes are \\, \n, \t, \u{hex} and \/` |

A group that took no part in a match copies nothing. The runtimes expand a replacement
by these pieces, never by their libraries' rewrite syntax.

## Doubts

- **CT** The runtimes needed one accessor each in a collections file, which another
  branch is rewriting: `Cursor::text()` (Rust `&Text`, C++ `const Text&`), the whole text
  a cursor is in, which `?Match` searches with its context. Nothing else in Text or
  Cursor changed; views are cut with the public `Cursor::at_byte`, `before` and
  `after`. *If otherwise:* the accessor moves wherever the rewrite puts Cursor.
- **PC** `\pC` (Rust includes the unassigned code points, RE2 does not), `\p{Cn}` and
  `\p{LC}` (RE2 has neither), `\p{Cs}` (Rust refuses it) are refused. *If otherwise:*
  `C` spelled as `[\p{Cc}\p{Cf}\p{Co}]` (RE2's reading) to both libraries.
- **UN** Rust's `regex-syntax` 0.8 carries Unicode 16.0's tables, Homebrew's RE2
  (2025-11-05) 15.x's; the books pin 15.0 (doubt UV). The seven scripts new in 16.0 are
  refused; categories and folding differ on the code points 16.0 assigned, which the
  corpus does not test. *If otherwise:* UV's alternative, tables of the runtimes' own.
- **CZ** A count with a leading zero is refused: RE2 reads `a{01}` as the text `a{01}`,
  Rust as `a{1}`, and the books say only that a brace is always repetition.
- **AR** A repetition of an assertion (`^*`, `\b+`) is refused; the books are silent and
  both libraries take it. `(?:^)*` says it.
- **NI** A negated set inside a class under `i` (`(?i)[\W]`, `(?i)[\P{Lu}]`,
  `(?i)[[:^alpha:]]`) is refused: the books' "every code point that folds to one of its
  members" does not say whether the negation is taken before or after the folding (it
  decides whether `(?i)[\W]` matches `k`, through U+212A). Outside a class the negation
  is after the folding, as `[^...]` is in both libraries.
- **WI** `(?i)\w` matches U+212A and U+017F, by the books' rule that `i` applies to
  classes; a lexer of ASCII names under `i` sees them. Pinned, not refused.
- **DP** The nesting bound counts groups and repetitions as the depth defined above; a
  bare `(?flags)` does not count. The early refusal (a 101st open group) gives the same
  diagnostic, at 0, so the two orders agree.
- **SZ** The size counts a class as one atom and a count by its factor; the books say
  "atoms, counted repetition expanded" and no more.
- **RZ** A replacement's group number with a leading zero is refused; the books say "all
  the digits that follow".
- **EM** The messages and positions are this file's; the books give the diagnostic's
  kind and its reason for backreferences and lookaround.
- **HR** The runtimes read a replacement's escapes the same in the plain and the hashed
  form (chapter 2 makes the hashed form's pattern raw and says nothing of its
  replacement); the lexer hands both over as written.
- **SH** A `RegexReplacement` prints as the plain literal even where only the hashed form
  could be read back (a replacement starting with letters and holding a space, doubt
  RX-R); nothing reads one back.
