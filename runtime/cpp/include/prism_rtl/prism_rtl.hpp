// `prism_rtl`: the runtime library compiled Prism programs link (Compiler ch. 5), in C++.
//
// Emitted code is typed C++: one C++ type per Prism type, typed closures, lifting
// resolved at compile time, and each widget's graph (its nodes, heights, propagation,
// holds, states and transitions) emitted from code templates. This library is what is
// left: the part of a compiled program that is not generated per block, written once
// and shared by every program. It is a port of the Rust crate `prism_rtl`
// (runtime/rust/), whose contract with the emitter is claude/backend.md; README.md
// beside this directory maps each Rust name to its C++ one.
//
// The headers, from the bottom up:
//
// - result.hpp: `Tick`, `Unit`, `Fault`, `Panic`, `Expected`/`Result`/`Done`, `TRY`.
// - rc.hpp: `Rc` and `Weak`, reference counting with plain counts (one thread).
// - num.hpp: `Nat` and `Int`, arbitrary precision with a small-value fast path.
// - collections.hpp: `Text`, `Map` and `Array`, immutable values copied on write.
// - text_case.hpp: `Upper` and `Lower`, Unicode's full case mapping.
// - pod.hpp: `Pod`, the plain data a World sees; paths; the `^Pod` traits; `Error` (a
//   `~T`'s failure).
// - print.hpp: the transcript's spelling, as the printer API derived `^Show` calls.
// - read.hpp: reading that spelling back, and the test language's patterns.
// - doc.hpp: the document and the patch diff; requests.
// - world.hpp: the protocol (`Program`, `TickIn`, `TickOut`, `ReadyState`, `World`,
//   `drive`).
// - gc.hpp: the zombie collector, trial deletion in the compact round.
// - cx.hpp: the `Node` interface, the tick context `Cx`, and the `Runtime` that walks
//   the lists in Graph's order.
// - cells.hpp: the node primitives: `Hold`, `Stamped`, `Cache`, `Snapshot`, `Port`,
//   `Event`, `Dynamic`, `Live`, `Defer`, and the `Job` an unbounded function's step
//   machine implements.
// - host.hpp: hosting: the rows of a host's `for` over its child's document, the
//   blueprint forms a supplier answers with, and the typed way back into the child.
// - prelude.hpp: `Prelude.Unwrap` and `Prelude.Exceptions` as every World answers them.
// - scene.hpp, test_world.hpp: the scene's clickables and the Test World.

#pragma once

#include "prism_rtl/cells.hpp"
#include "prism_rtl/collections.hpp"
#include "prism_rtl/cx.hpp"
#include "prism_rtl/doc.hpp"
#include "prism_rtl/host.hpp"
#include "prism_rtl/gc.hpp"
#include "prism_rtl/num.hpp"
#include "prism_rtl/pod.hpp"
#include "prism_rtl/prelude.hpp"
#include "prism_rtl/print.hpp"
#include "prism_rtl/read.hpp"
#include "prism_rtl/regex.hpp"
#include "prism_rtl/result.hpp"
#include "prism_rtl/scene.hpp"
#include "prism_rtl/test_world.hpp"
#include "prism_rtl/text_case.hpp"
#include "prism_rtl/world.hpp"
