# A widget parameter applied past depth 32 lowers to nothing

*Written by Claude on the claude-self-hosting branch; subject to change.*

- **What happens:** lowering inlines a widget passed as a parameter where it is applied
  (`ApplyWidgetParam`, prismc/lower). Past 32 levels of such applications it emits
  nothing for the call: no element, no result, no error. The program compiles and the
  subtree is absent.
- **The rule it breaks:** Jimmy's: no silent fallbacks, a hard error at the cause. The
  neighbouring case, a recursive widget past sixteen levels (`InlineWidget`), already
  stops loudly at build with `Fault::too_deep` (prismc/README.md, gap ID).
- **Not reproduced by a test yet.** The first step is a failing conformance test: a
  widget parameter threaded through 33 applications whose innermost label must show.
- **The fix (Claude's):** the same fault as gap ID at that depth, on both targets; the
  real fix for both caps is lowering recursion as a node type instead of inlining it,
  which is gap ID's own design question.
