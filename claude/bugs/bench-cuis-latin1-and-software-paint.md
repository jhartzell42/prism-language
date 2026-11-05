# The Cuis table crashes on `▶`, and on Linux its times measure software painting

*Written by Claude on the claude-self-hosting branch, 5 November 2026; subject to change.
Two problems in `bench/gui/cuis/`, found running the bench on the cloud box (Linux,
Xvfb, the Linux Cog VM the Cuis repository ships beside the Mac one).*

## Select crashes

- **The scenario:** `cuis/run.sh select` stops at the timed click with
  `Error: String only store Latin-1 Characters`.
- **The cause:** the select marker `▶` (U+25B6) is concatenated into a label held in a
  plain `String`, which in Cuis holds Latin-1 only.
- **The fix:** build the marked label as a `UnicodeString` (or the marker as a
  `UnicodeString` and the concatenation from it), so the label can hold it. Check that
  `update10th`'s `' !!!'` path and the row labels still compare equal for the driver's
  check after the change.

## On Linux under Xvfb, the times follow how much of the window gets repainted

Medians of 5 runs, ms:

| op | Cuis | what gets painted |
|---|---|---|
| create1k | 7,159 | the whole window |
| replace1k | 17,869 | the whole window |
| remove | 8,733 | every visible row moves up |
| append1k | 21 | nothing new: the rows land below the fold |
| update10th | 1 | a few visible labels |
| swap | 27 | one visible row |

Creating a thousand rows by appending takes 21 ms, and creating them into an empty
table takes 7 s, so the 7 s is drawing, not building Morphs. The likeliest reading:
VectorEngine paints in software on this VM under Xvfb, at a few seconds per full
window. Not a bug to fix in the bench; the RESULTS for Cuis should come from the Mac's
screen, and a Linux run should say this beside its Cuis column.
