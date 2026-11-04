// The CLI World in C++: the C++ form of worlds/cli/rust (the Rust crate `prism_world_cli`), the
// World compiled programs run under at the command line, over `prism_rtl`'s protocol. It
// answers `Fs`, `Process` and `Time`, the prelude's `Prelude.Unwrap` and
// `Prelude.Exceptions`, and `Fs.List`; worlds/cli/rust/README.md says what each answers, and
// holds here word for word.
//
//   prism_world_cli::run<MyProgram>(args)       // the real file system, at the working directory
//   CliWorld world(std::make_unique<MemFs>(), args); prism_rtl::drive<MyProgram>(world, {}, patience);
//
// Written by Claude, 4 November 2026, on the claude-self-hosting branch. Subject to change.

#pragma once

#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <map>
#include <memory>
#include <set>
#include <string>
#include <vector>

#include "prism_rtl/prism_rtl.hpp"

namespace prism_world_cli {

using prism_rtl::Document;
using prism_rtl::Error;
using prism_rtl::Expected;
using prism_rtl::Path;
using prism_rtl::Pod;

/// The families this World answers.
inline const std::vector<std::string> FAMILIES = {"Fs", "Process", "Time", "Prelude"};

/// What `Fs.*` reads and writes: the disk, or memory in tests.
class FileSystem {
public:
    virtual ~FileSystem() = default;
    virtual Expected<std::string, Error> read(const std::string& path) const = 0;
    virtual Expected<void, Error> write(const std::string& path, const std::string& text) = 0;
    /// Every `.prism` and `.prismtest` file under `dir`, recursively, sorted, each path
    /// starting with `dir` as written; hidden entries skipped.
    virtual std::vector<std::string> list(const std::string& dir) const = 0;
};

/// Whether a listing takes a file.
bool listed(const std::string& name);
/// `dir` joined to a path under it, as written (`.` and `` leave it out).
std::string joined(const std::string& dir, const std::string& rest);

/// The disk under `root`; a write goes beside the target, then is renamed onto it.
class RealFs : public FileSystem {
public:
    explicit RealFs(std::string root) : root_(std::move(root)) {}
    Expected<std::string, Error> read(const std::string& path) const override;
    Expected<void, Error> write(const std::string& path, const std::string& text) override;
    std::vector<std::string> list(const std::string& dir) const override;

private:
    std::string at(const std::string& path) const;
    std::string root_;
};

/// Files in memory, by path.
class MemFs : public FileSystem {
public:
    std::map<std::string, std::string> files;
    Expected<std::string, Error> read(const std::string& path) const override;
    Expected<void, Error> write(const std::string& path, const std::string& text) override;
    std::vector<std::string> list(const std::string& dir) const override;
};

/// The prelude's part of a World: `Prelude.Unwrap` and `Prelude.Exceptions` (Semantics
/// ch. 10). An unwrap of a value answers it; of an `Error` poisons the unwrap's subtree
/// and reports the exception to the nearest supervisor outside it, or gives up the
/// program.
class Prelude {
public:
    bool is_poisoned(const Path& p) const;
    void track(const prism_rtl::Patch& patch);
    void unwrap(const Path& path, const Pod& value);
    std::vector<std::pair<Path, Pod>> take_answers();
    void filter(prism_rtl::TickIn& input, const Document& doc) const;
    std::optional<std::uint8_t> exit() const { return exit_; }

private:
    std::vector<Path> poisoned_;
    std::set<Path> exceptions_;
    std::vector<std::pair<Path, Pod>> answers_;
    std::optional<std::uint8_t> exit_;
};

/// The subtree an unwrap stands in: up to its last generation or key segment.
Path subtree_of(const Path& p);
/// The element at a path in the document, or added by this tick's patch.
const Pod* element_of(const Document& doc, const prism_rtl::TickOut& out, const Path& path);

/// The CLI World.
class CliWorld : public prism_rtl::World {
public:
    CliWorld(std::unique_ptr<FileSystem> fs, std::vector<std::string> args);
    std::shared_ptr<const prism_rtl::ReadyState> ready() const override;
    void perform(const prism_rtl::TickOut& out) override;
    std::optional<prism_rtl::TickIn> next_inputs() override;
    std::optional<std::uint8_t> exit_code() const override;
    FileSystem& fs() { return *fs_; }

private:
    using Clock = std::chrono::steady_clock;
    struct Timer {
        Clock::time_point at;
        Path path;
        Pod value;
    };
    Expected<void, Error> write(const std::string& path, const std::string& text);
    void request(const Pod& element, const Path& path, const Pod& value);
    void output(const Pod& element);
    void relist();
    void due();

    std::shared_ptr<FileSystem> fs_;
    std::shared_ptr<const std::vector<std::string>> args_;
    Document doc_;
    std::vector<std::pair<Path, Pod>> answers_;
    std::vector<std::pair<Pod, Pod>> pushes_;
    std::vector<Timer> timers_;
    std::map<Pod, Pod> lists_;
    Prelude prelude_;
    bool pending_ = false;
    std::optional<std::uint8_t> exit_;
};

/// A run's patience by default: unbounded (`PRISM_PATIENCE` sets one).
inline constexpr std::size_t RUN_PATIENCE = std::size_t(-1);

/// The program `P` under the CLI World over the real file system at the working
/// directory, with the command line's arguments.
template <class P>
int run(const std::vector<std::string>& args) {
    CliWorld world(std::make_unique<RealFs>("."), args);
    std::size_t patience = RUN_PATIENCE;
    if (const char* p = std::getenv("PRISM_PATIENCE")) patience = std::strtoull(p, nullptr, 10);
    return prism_rtl::drive<P>(world, {}, patience);
}

}  // namespace prism_world_cli
