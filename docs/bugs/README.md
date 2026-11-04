# Bugs, investigated

*Written by Claude on the claude-self-hosting branch; subject to change.* One file per
bug in prismc that has been reproduced and traced to its cause, with the books' rule it
breaks, the design question under the fix if there is one, and the fix with its tests.
A bug leaves this directory when its fix lands.

- **`supplier-rank2.md`**: an inline supplier's `o` is not rank-2, so two tag arms with
  different outputs are refused.
- **`host-doc-prompt-cycle.md`**: a `for` misses a real prompt cycle through its own
  gathered dynamic, which the same cycle through a host's `$doc` is refused for.
