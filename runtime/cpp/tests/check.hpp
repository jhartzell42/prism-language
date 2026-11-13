// A tiny test harness: `TEST(name) { ... }` registers a test; `CHECK(cond)` and
// `CHECK_EQ(a, b)` record a failure and go on. The runner (main.cpp) runs every test,
// or those whose names contain an argument, and exits 1 on any failure.

#pragma once

#include <functional>
#include <iostream>
#include <sstream>
#include <string>
#include <vector>

namespace check {

struct Test {
    const char* name;
    std::function<void()> body;
};

inline std::vector<Test>& registry() {
    static std::vector<Test> tests;
    return tests;
}

inline int& failures() {
    static int n = 0;
    return n;
}

struct Register {
    Register(const char* name, std::function<void()> body) { registry().push_back({name, std::move(body)}); }
};

inline void fail(const char* file, int line, const std::string& what) {
    ++failures();
    std::cerr << file << ":" << line << ": " << what << "\n";
}

template <class T>
std::string shown(const T& x) {
    if constexpr (requires(std::ostream& o) { o << x; }) {
        std::ostringstream o;
        o << x;
        return o.str();
    } else {
        return "<value>";
    }
}

}  // namespace check

#define CHECK_CAT2(a, b) a##b
#define CHECK_CAT(a, b) CHECK_CAT2(a, b)
#define TEST(name)                                                                  \
    static void name();                                                             \
    static ::check::Register CHECK_CAT(register_, name)(#name, name);               \
    static void name()

#define CHECK(...)                                                                    \
    do {                                                                              \
        if (!(__VA_ARGS__)) ::check::fail(__FILE__, __LINE__, "CHECK(" #__VA_ARGS__ ")"); \
    } while (0)

#define CHECK_EQ(a, b)                                                                                     \
    do {                                                                                                   \
        auto check_a_ = (a);                                                                             \
        auto check_b_ = (b);                                                                             \
        if (!(check_a_ == check_b_))                                                                       \
            ::check::fail(__FILE__, __LINE__,                                                              \
                          "CHECK_EQ(" #a ", " #b ")\n  left:  " + ::check::shown(check_a_) + "\n  right: " + \
                              ::check::shown(check_b_));                                                   \
    } while (0)
