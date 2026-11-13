// The allocator the test binary runs on: jemalloc's, unless the build
// is a sanitizer's, whose AddressSanitizer keeps its own. A link that silently kept the C
// library's malloc fails here.
//
// Written by Claude on the claude-cpp-rc-jemalloc branch; subject to change.

#include <dlfcn.h>

#include <cstdint>
#include <cstdlib>

#include "check.hpp"

#ifndef PRISM_TESTS_JEMALLOC
#error "the Makefile says whether this build links jemalloc (PRISM_TESTS_JEMALLOC)"
#endif

namespace {

TEST(allocator_is_jemalloc_outside_sanitizers) {
    // jemalloc's own entry point, present in the process exactly when it is linked
    using Mallctl = int (*)(const char*, void*, std::size_t*, void*, std::size_t);
    auto mallctl = reinterpret_cast<Mallctl>(dlsym(RTLD_DEFAULT, "mallctl"));
    if (!PRISM_TESTS_JEMALLOC) {
        CHECK(mallctl == nullptr);
        return;
    }
    CHECK(mallctl != nullptr);
    if (!mallctl) return;
    // and it is the malloc the program calls: a block malloc gives is one jemalloc counts
    const char* version = nullptr;
    std::size_t len = sizeof(version);
    CHECK_EQ(mallctl("version", &version, &len, nullptr, 0), 0);
    std::size_t before = 0, after = 0, n = sizeof(std::size_t);
    std::uint64_t epoch = 1;
    std::size_t elen = sizeof(epoch);
    mallctl("epoch", &epoch, &elen, &epoch, elen);
    CHECK_EQ(mallctl("stats.allocated", &before, &n, nullptr, 0), 0);
    void* p = std::malloc(1 << 20);
    // the block escapes to an opaque use, so that no compiler elides the pair (clang folds
    // a malloc whose block nothing reads)
    asm volatile("" : : "r"(p) : "memory");
    mallctl("epoch", &epoch, &elen, &epoch, elen);
    CHECK_EQ(mallctl("stats.allocated", &after, &n, nullptr, 0), 0);
    CHECK(after >= before + (1 << 20));
    std::free(p);
}

}  // namespace
