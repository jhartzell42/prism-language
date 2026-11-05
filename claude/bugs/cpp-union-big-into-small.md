# C++ `Union` copies the small map and inserts the big one: quadratic folds

*Written by Claude on the claude-self-hosting branch, 5 November 2026; subject to change.
A performance bug in the C++ prelude: the results are right, and a fold of `Union` is
quadratic.*

## The scenario

The GUI table's create10k (`bench/gui/`, cloud box, 5 November), medians:

| | create1k | create10k | ratio |
|---|---|---|---|
| Prism C++ | 1,055 ms | 118,543 ms | 112× |
| Prism Rust | 717 ms | 13,694 ms | 19× |
| GTK | 482 ms | 35,729 ms | 74× |

Ten times the rows costs C++ 112 times the time, and C++ is nine times slower than Rust
here while faster at every other operation but create1k and append1k.

## The cause

- **The profile:** gdb stack samples of C++ create10k, 30 of 30 under
  `window::texts_in`, 27 of them in `prelude::union_` and `Map::put` / `balance`.
- **The fold:** `TextsIn` (examples/chess/window.prism:486) is
  `Fold (Values cs), {}, \|acc, e| Union acc, (TextsOf e)`: the accumulator grows to every
  word in the window, and each step unions one row's few words into it.
- **C++'s `union_`** (`prismc/templates/cpp_prelude.prism`, emitted as `prelude.hpp`):

  ```cpp
  Map<K, V> union_(const Map<K, V>& a, const Map<K, V>& b) {
      Map<K, V> m = b;
      for (const auto& [k, v] : a) m.insert(k, v);
      return m;
  }
  ```

  It copies `b`, the small map, and inserts every entry of `a`, the accumulator: each
  step costs the whole accumulator, so the fold is O(n² log n).
- **Rust's `union`** is `b.clone().union(a.clone())`, and `im_rc::OrdMap::union` inserts
  the smaller map into the larger, so the same fold is O(n log n).

## Is it a bug?

Yes. Appendix B's cost promises for `Union` cannot be met by inserting the bigger map
entry by entry, and the two targets disagree by an order of magnitude on one program.

## The fix

- **Insert the smaller into the larger,** keeping `Union`'s bias (`a`'s value wins on a
  shared key): when `a` is larger, start from `a` and insert each of `b`'s entries only
  where the key is absent; otherwise start from `b` and insert `a`'s entries over it.
  Both are persistent copies, so starting from the larger one shares its tree.
- **Check its neighbours** in the same template for the same shape: any function that
  copies one argument and inserts the other (`Intersect`, `Difference`, `filter_map`).
- **Tests:** a prelude test that `Union` keeps `a`'s value on a shared key in both size
  orders, and a timing guard in the bench, not the suite: C++ create10k within about 15×
  create1k, as Rust is.
- **Independent of** `window-relayout-every-change.md`, which also removes this fold from
  the hot path; fix both, since any program folding `Union` hits this.
