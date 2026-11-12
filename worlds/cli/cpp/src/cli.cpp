// The CLI World (cli.hpp): the file systems, the prelude's unwraps, the World itself.
//
// Written by Claude, 4 November 2026, on the claude-self-hosting branch. Subject to change.

#include "prism_world_cli/cli.hpp"

#include <algorithm>
#include <cerrno>
#include <cstdlib>
#include <cstdio>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <sstream>
#include <thread>

namespace prism_world_cli {

namespace sfs = std::filesystem;
using prism_rtl::Segment;
using prism_rtl::TickIn;
using prism_rtl::TickOut;
using prism_rtl::unexpected;

// ---- File systems -----------------------------------------------------------------------

bool listed(const std::string& name) {
    auto ends = [&](const std::string& s) { return name.size() >= s.size() && name.compare(name.size() - s.size(), s.size(), s) == 0; };
    return ends(".prism") || ends(".prismtest");
}

std::string joined(const std::string& dir, const std::string& rest) {
    std::string d = dir;
    while (!d.empty() && d.back() == '/') d.pop_back();
    if ((d.empty() || d == ".") && (dir.empty() || dir[0] != '/')) return rest;
    return d + "/" + rest;
}

namespace {

Error io_error(int err, const std::string& path) {
    const char* kind = err == ENOENT ? "Fs.NotFound" : (err == EACCES || err == EPERM) ? "Fs.Denied" : "Fs.Failed";
    return Error(kind, prism_rtl::Text(path + ": " + std::strerror(err)));
}

Error fs_error(const std::error_code& ec, const std::string& path) {
    const char* kind = ec == std::errc::no_such_file_or_directory ? "Fs.NotFound"
                       : ec == std::errc::permission_denied        ? "Fs.Denied"
                                                                   : "Fs.Failed";
    return Error(kind, prism_rtl::Text(path + ": " + ec.message()));
}

void walk(const sfs::path& at, const std::string& prefix, std::vector<std::string>& out) {
    std::error_code ec;
    sfs::directory_iterator it(at, ec);
    if (ec) return;
    for (const auto& entry : it) {
        std::string name = entry.path().filename().string();
        if (!name.empty() && name[0] == '.') continue;
        std::string rel = prefix.empty() ? name : prefix + "/" + name;
        std::error_code e2;
        if (entry.is_directory(e2)) walk(entry.path(), rel, out);
        else if (listed(name) && entry.is_regular_file(e2)) out.push_back(rel);
    }
}

}  // namespace

std::string RealFs::at(const std::string& path) const {
    if (!path.empty() && path[0] == '/') return path;
    return root_ == "." ? path : root_ + "/" + path;
}

Expected<std::string, Error> RealFs::read(const std::string& path) const {
    std::ifstream f(at(path), std::ios::binary);
    if (!f) return unexpected(io_error(errno ? errno : ENOENT, path));
    std::ostringstream s;
    s << f.rdbuf();
    std::string bytes = s.str();
    if (auto bad = prism_rtl::utf8_error_at(bytes))
        return unexpected(Error("Fs.NotText", prism_rtl::Text(path + ": not UTF-8: byte " + std::to_string(*bad) + " is malformed")));
    return bytes;
}

Expected<void, Error> RealFs::write(const std::string& path, const std::string& text) {
    sfs::path target = at(path);
    std::error_code ec;
    if (target.has_parent_path()) {
        sfs::create_directories(target.parent_path(), ec);
        if (ec) return unexpected(fs_error(ec, path));
    }
    std::string tmp = target.string() + ".prism-tmp";
    {
        std::ofstream f(tmp, std::ios::binary | std::ios::trunc);
        if (!f) return unexpected(io_error(errno ? errno : EIO, path));
        f << text;
        if (!f) return unexpected(io_error(EIO, path));
    }
    sfs::rename(tmp, target, ec);
    if (ec) return unexpected(fs_error(ec, path));
    return {};
}

std::vector<std::string> RealFs::list(const std::string& dir) const {
    std::vector<std::string> rel;
    walk(at(dir.empty() ? "." : dir), "", rel);
    std::vector<std::string> out;
    for (const auto& r : rel) out.push_back(joined(dir, r));
    std::sort(out.begin(), out.end());
    return out;
}

Expected<std::string, Error> MemFs::read(const std::string& path) const {
    auto it = files.find(path);
    if (it == files.end()) return unexpected(Error("Fs.NotFound", prism_rtl::Text(path + ": no such file")));
    return it->second;
}

Expected<void, Error> MemFs::write(const std::string& path, const std::string& text) {
    files[path] = text;
    return {};
}

std::vector<std::string> MemFs::list(const std::string& dir) const {
    std::string d = dir;
    while (!d.empty() && d.back() == '/') d.pop_back();
    std::vector<std::string> out;
    for (const auto& [p, text] : files) {
        bool inside = d.empty() || d == "." || (p.size() > d.size() && p.compare(0, d.size(), d) == 0 && p[d.size()] == '/');
        std::string name = p.substr(p.rfind('/') == std::string::npos ? 0 : p.rfind('/') + 1);
        if (inside && listed(name) && name[0] != '.') out.push_back(p);
    }
    std::sort(out.begin(), out.end());
    return out;
}

// ---- The World --------------------------------------------------------------------------

namespace {

Pod contents(const Expected<std::string, Error>& r) {
    if (r) return Pod::ok(Pod::record("Contents", {{"text", Pod::text(prism_rtl::Text(*r))}}));
    return Pod::failed(prism_rtl::to_pod(r.error()));
}

Pod listing(const std::vector<std::string>& paths) {
    std::vector<Pod> out;
    for (const auto& p : paths) out.push_back(Pod::text(prism_rtl::Text(p)));
    return Pod::array(std::move(out));
}

std::optional<std::string> text_field(const Pod& e, const char* name) {
    if (auto f = e.field(name))
        if (auto t = f->as_text()) return t->to_string();
    return std::nullopt;
}

bool tagged(const Pod& e, const char* tag) {
    auto t = e.tag();
    return t && *t == tag;
}

}  // namespace

// `Process.Env`'s answer: `? value`, or `nil` when the variable is unset or not UTF-8. The
// value is copied out of the environment at once; nothing in the program changes the
// environment, so the pointer `getenv` gives is read before anything could invalidate it.
Pod env_value(const std::string& name) {
    const char* v = std::getenv(name.c_str());
    if (!v) return Pod::nil();
    std::string s(v);
    if (prism_rtl::utf8_error_at(s)) return Pod::nil();
    return Pod::some(Pod::text(prism_rtl::Text(s)));
}

namespace {

struct CliReady : prism_rtl::ReadyState {
    prism_rtl::Rc<FileSystem> fs;
    prism_rtl::Rc<const std::vector<std::string>> args;
    std::optional<Pod> first_value(const Pod& e) const override {
        auto tag = e.tag();
        if (!tag) return std::nullopt;
        if (*tag == "Fs.Contents") {
            auto p = text_field(e, "path");
            if (!p) return std::nullopt;
            return contents(fs->read(*p));
        }
        if (*tag == "Fs.Output") return Pod::name("Writing");
        if (*tag == "Fs.List") {
            auto d = text_field(e, "dir");
            if (!d) return std::nullopt;
            return listing(fs->list(*d));
        }
        if (*tag == "Process.Args") return listing(*args);
        if (*tag == "Process.Env") {
            auto n = text_field(e, "name");
            if (!n) return std::nullopt;
            return env_value(*n);
        }
        return std::nullopt;
    }
};

}  // namespace

CliWorld::CliWorld(std::unique_ptr<FileSystem> fs, std::vector<std::string> args)
    : fs_(std::move(fs)), args_(prism_rtl::make_rc<const std::vector<std::string>>(std::move(args))) {}

prism_rtl::Rc<const prism_rtl::ReadyState> CliWorld::ready() const {
    auto r = prism_rtl::make_rc<CliReady>();
    r->fs = fs_;
    r->args = args_;
    return r;
}

Expected<void, Error> CliWorld::write(const std::string& path, const std::string& text) {
    auto w = fs_->write(path, text);
    if (!w) return w;
    Pod watched = Pod::element("Fs.Contents", {{"path", Pod::text(prism_rtl::Text(path))}});
    pushes_.emplace_back(watched, contents(text));
    return {};
}

void CliWorld::request(const Pod& element, const Path& path, const Pod& value) {
    if (tagged(element, "Prelude.Unwrap")) {
        prelude_.unwrap(path, value);
        return prelude_.report_gave_up();
    }
    if (tagged(element, "Process.Exit")) {
        if (!exit_) {
            auto code = prism_rtl::from_pod<std::uint8_t>(value);
            exit_ = code ? *code : prism_rtl::EXIT_FAILED;
        }
        return;
    }
    if (tagged(element, "Time.After")) {
        auto after = std::chrono::nanoseconds(0);
        if (auto f = value.field("after"))
            if (auto ns = f->structure().if_duration())
                after = std::chrono::nanoseconds(std::int64_t(std::clamp<prism_rtl::Nanos>(*ns, 0, INT64_MAX)));
        timers_.push_back(Timer{Clock::now() + after, path, value});
        return;
    }
    const Pod::Choice* c = value.structure().if_choice();
    if (tagged(element, "Fs.File") && c && c->ctor == "Read") {
        auto file = text_field(element, "path");
        if (!file) return;
        answers_.emplace_back(path, contents(fs_->read(*file)));
        return;
    }
    if (tagged(element, "Fs.File") && c && c->ctor == "Write") {
        auto file = text_field(element, "path");
        if (!file) return;
        std::string text;
        if (!c->args.empty())
            if (auto t = c->args[0].as_text()) text = t->to_string();
        auto w = write(*file, text);
        answers_.emplace_back(path, w ? contents(text) : contents(unexpected(w.error())));
    }
}

void CliWorld::output(const Pod& element) {
    auto path = text_field(element, "path");
    auto text = text_field(element, "text");
    if (!path || !text) return;
    auto w = write(*path, *text);
    Pod state = w ? Pod::name("Written") : Pod::choice("WriteFailed", {prism_rtl::to_pod(w.error())});
    pushes_.emplace_back(element, state);
}

void CliWorld::relist() {
    for (auto& [element, last] : lists_) {
        auto dir = text_field(element, "dir");
        if (!dir) continue;
        Pod now = listing(fs_->list(*dir));
        if (!(now == last)) {
            last = now;
            pushes_.emplace_back(element, now);
        }
    }
}

void CliWorld::due() {
    auto now = Clock::now();
    std::vector<Timer> later;
    for (auto& t : timers_) {
        if (t.at <= now) answers_.emplace_back(t.path, t.value);
        else later.push_back(t);
    }
    timers_ = std::move(later);
}

void CliWorld::perform(const TickOut& out) {
    pending_ = out.pending;
    prelude_.track(out.patch);
    for (const auto& r : out.requests) {
        if (prelude_.is_poisoned(r.path)) continue;
        const Pod* element = element_of(doc_, out, r.path);
        if (!element) continue;
        Pod e = *element;
        request(e, r.path, r.value);
    }
    doc_.apply(out.patch);
    for (const auto& [p, e] : out.patch.removed) {
        if (!tagged(e, "Fs.List")) continue;
        bool still = std::any_of(doc_.begin(), doc_.end(), [&](const auto& qf) { return !(qf.first == p) && qf.second == e; });
        if (!still) lists_.erase(e);
    }
    for (const auto& [p, e] : out.patch.added) {
        if (tagged(e, "Fs.List") && !prelude_.is_poisoned(p)) {
            auto first = ready()->first_value(e);
            lists_.try_emplace(e, first ? *first : Pod::array({}));
        }
    }
    std::vector<Pod> writes;
    for (const auto* edits : {&out.patch.added, &out.patch.changed})
        for (const auto& [p, e] : *edits)
            if (tagged(e, "Fs.Output") && !prelude_.is_poisoned(p)) writes.push_back(e);
    bool wrote = !writes.empty() || !pushes_.empty();
    for (const auto& e : writes) output(e);
    if (wrote && !lists_.empty()) relist();
}

std::optional<TickIn> CliWorld::next_inputs() {
    while (true) {
        due();
        for (auto& a : prelude_.take_answers()) answers_.push_back(std::move(a));
        if (answers_.empty() && pushes_.empty() && !pending_ && !timers_.empty()) {
            auto at = std::min_element(timers_.begin(), timers_.end(), [](const Timer& a, const Timer& b) { return a.at < b.at; })->at;
            std::this_thread::sleep_until(at);
            due();
        }
        TickIn input;
        input.answers = std::exchange(answers_, {});
        input.pushes = std::exchange(pushes_, {});
        prelude_.filter(input, doc_);
        if (!input.answers.empty() || !input.pushes.empty()) return input;
        if (timers_.empty() || pending_) return std::nullopt;
    }
}

std::optional<std::uint8_t> CliWorld::exit_code() const { return exit_ ? exit_ : prelude_.exit(); }

}  // namespace prism_world_cli
