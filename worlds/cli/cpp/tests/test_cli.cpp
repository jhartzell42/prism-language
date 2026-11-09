// The CLI World in C++ against programs written by hand in the shape claude/backend.md gives
// the templates, driven by `prism_rtl::drive`: the C++ form of worlds/cli/rust/tests/cli.rs and
// the unit tests of its fs.rs and prelude.rs, test for test.
//
// Written by Claude, 4 November 2026, on the claude-self-hosting branch. Subject to change.

#include <chrono>
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <unistd.h>

#include "check.hpp"
#include "prism_world_cli/cli.hpp"

using namespace prism_rtl;
using namespace prism_world_cli;
namespace sfs = std::filesystem;

namespace {

// ---- A program around one root node -------------------------------------------------------

template <class N>
struct Prog : Program {
    Runtime rt;
    prism_rtl::Rc<N> root;
    static Built<Prog> build(std::span<const Pod>, const ReadyState& ready) {
        auto p = std::make_unique<Prog>();
        Cx cx = p->rt.build(ready);
        auto made = N::construct(Path::root(), cx);
        if (!made) return unexpected(made.error());
        p->root = *made;
        auto out = p->rt.close(std::move(cx));
        if (!out) return unexpected(out.error());
        return std::pair<std::unique_ptr<Prog>, TickOut>(std::move(p), std::move(out).value());
    }
    Result<TickOut> tick(const TickIn& input, const ReadyState& ready) override { return rt.tick(root, input, ready); }
    void compact() override { rt.compact(); }
};

Done no_place(std::span<const Segment> at) { return unexpected(Fault::answer("no answering place at " + path_of(at).to_string())); }

Pod text(const std::string& s) { return Pod::text(Text(s)); }

// ---- copy.prism (worlds/cli/rust/tests/cli.rs) ---------------------------------------------------

struct Copy : Node {
    Path path;
    Text to;
    Hold<Text> text;
    Copy(Path path, Text to, Hold<Text> text) : path(std::move(path)), to(std::move(to)), text(std::move(text)) {}
    Pod output(const Cx& cx) const { return Pod::element("Fs.Output", {{"path", to_pod(to)}, {"text", to_pod(text.get(cx))}}); }
    static Result<prism_rtl::Rc<Copy>> construct(Path path, Cx& cx) {
        Pod args_el = Pod::element("Process.Args", {});
        auto first = cx.first_value(args_el);
        if (!first) return unexpected(Fault::answer("Process.Args unanswered"));
        auto args = from_pod<Array<Text>>(*first);
        if (!args) return unexpected(args.error());
        auto node = prism_rtl::make_rc<Copy>(std::move(path), (*args)[1], Hold<Text>(Text(""), cx));
        cx.added(node->path.at(0), args_el);
        cx.added(node->path.at(1), Pod::element("Fs.File", {{"path", to_pod((*args)[0])}}));
        cx.added(node->path.at(2), Pod::element("Prelude.Unwrap", {}));
        Pod out = node->output(cx);
        CHECK(cx.first_value(out) == std::optional<Pod>(Pod::name("Writing")));
        cx.added(node->path.at(3), out);
        cx.listen(out, node);
        cx.added(node->path.at(4), Pod::element("Process.Exit", {}));
        cx.on_started(node);
        return node;
    }
    Done started(Cx& cx) override {
        cx.request(path.at(1), std::nullopt, Pod::name("Read"));
        return {};
    }
    Done answer(std::span<const Segment> at, const Pod& v, Cx& cx) override {
        if (at.size() == 1 && at[0].is_pos(1)) {
            cx.request(path.at(2), std::nullopt, v);
            return {};
        }
        if (at.size() == 1 && at[0].is_pos(2)) {
            const Pod* t = v.field("text");
            if (!t || !t->as_text()) return unexpected(Fault::answer("expected Contents"));
            std::string s = t->as_text()->to_string();
            for (auto& c : s) c = char(std::toupper(static_cast<unsigned char>(c)));
            text.set(Text(s), cx);
            cx.dirty(me());
            return {};
        }
        return no_place(at);
    }
    Done push(const Pod&, const Pod& v, Cx& cx) override {
        if (v == Pod::name("Written") && !text.get(cx).is_empty()) cx.request(path.at(4), std::nullopt, to_pod(std::uint8_t(42)));
        return {};
    }
    Done patch(Cx& cx) override {
        Pod level = output(cx);
        cx.changed(path.at(3), level);
        cx.listen(level, me());
        return {};
    }
};

std::unique_ptr<MemFs> mem(std::initializer_list<std::pair<const std::string, std::string>> files) {
    auto fs = std::make_unique<MemFs>();
    fs->files = files;
    return fs;
}

sfs::path temp_dir(const std::string& name) {
    sfs::path dir = sfs::temp_directory_path() / ("prism-world-cli-cpp-" + name + "-" + std::to_string(getpid()));
    sfs::remove_all(dir);
    sfs::create_directories(dir);
    return dir;
}

void write_file(const sfs::path& p, const std::string& s) {
    sfs::create_directories(p.parent_path());
    std::ofstream(p) << s;
}

std::string read_file(const sfs::path& p) {
    std::ifstream f(p);
    return std::string(std::istreambuf_iterator<char>(f), {});
}

}  // namespace

TEST(an_environment_variable_is_answered_plain_or_nil) {
    CliWorld world(mem({}), {});
    auto ready = world.ready();
    auto env = [&](const std::string& n) { return ready->first_value(Pod::element("Process.Env", {{"name", text(n)}})); };
    const char* path = std::getenv("PATH");
    CHECK(path != nullptr);
    if (path) CHECK(env("PATH") == std::optional<Pod>(Pod::some(text(path))));
    CHECK(env("PRISM_TEST_SURELY_UNSET_VARIABLE") == std::optional<Pod>(Pod::nil()));
}

TEST(a_program_reads_a_file_writes_one_and_exits_42) {
    CliWorld world(mem({{"in.txt", "hello"}}), {"in.txt", "out/up.txt"});
    CHECK_EQ(int(drive<Prog<Copy>>(world, {}, 100)), 42);
    CHECK_EQ(static_cast<MemFs&>(world.fs()).files["out/up.txt"], std::string("HELLO"));
}

TEST(a_missing_file_poisons_the_root_and_exits_1) {
    CliWorld world(mem({}), {"in.txt", "out/up.txt"});
    CHECK_EQ(int(drive<Prog<Copy>>(world, {}, 100)), 1);
    CHECK_EQ(static_cast<MemFs&>(world.fs()).files["out/up.txt"], std::string(""));
}

TEST(the_real_file_system_writes_beside_and_renames) {
    sfs::path dir = temp_dir("copy");
    write_file(dir / "in.txt", "real");
    CliWorld world(std::make_unique<RealFs>(dir.string()), {"in.txt", "out/up.txt"});
    CHECK_EQ(int(drive<Prog<Copy>>(world, {}, 100)), 42);
    CHECK_EQ(read_file(dir / "out/up.txt"), std::string("REAL"));
    CHECK(!sfs::exists(dir / "out/up.txt.prism-tmp"));
    sfs::remove_all(dir);
}

TEST(a_file_that_is_not_utf8_reads_as_fs_not_text) {
    sfs::path dir = temp_dir("not-text");
    write_file(dir / "bad.txt", "ab\xFF" "c");
    auto got = RealFs(dir.string()).read("bad.txt");
    CHECK(!got.has_value());
    CHECK_EQ(got.error().kind_p, std::string("Fs.NotText"));
    CHECK_EQ(got.error().message_p.to_string(), std::string("bad.txt: not UTF-8: byte 2 is malformed"));
    sfs::remove_all(dir);
}

// ---- Fs.List ----------------------------------------------------------------------------------

TEST(a_listing_is_recursive_sorted_and_only_prism_files) {
    sfs::path dir = temp_dir("list");
    for (const char* f : {"tests/b.prismtest", "tests/b.expected", "a.prism", "tests/deep/c.prism", "notes.md", ".hidden/x.prism", "z.prism"})
        write_file(dir / f, "");
    CliWorld world(std::make_unique<RealFs>(dir.string()), {});
    auto list = [&](const char* d) { return world.ready()->first_value(Pod::element("Fs.List", {{"dir", text(d)}})); };
    auto texts = [](std::initializer_list<const char*> xs) {
        std::vector<Pod> out;
        for (const char* x : xs) out.push_back(text(x));
        return std::optional<Pod>(Pod::array(out));
    };
    CHECK(list(".") == texts({"a.prism", "tests/b.prismtest", "tests/deep/c.prism", "z.prism"}));
    CHECK(list("tests") == texts({"tests/b.prismtest", "tests/deep/c.prism"}));
    CHECK(list("tests/") == texts({"tests/b.prismtest", "tests/deep/c.prism"}));
    CHECK(list("missing") == texts({}));
    sfs::remove_all(dir);
}

namespace {

struct Lister : Node {
    Path path;
    Hold<Pod> files;
    Lister(Path path, Hold<Pod> files) : path(std::move(path)), files(std::move(files)) {}
    static Result<prism_rtl::Rc<Lister>> construct(Path path, Cx& cx) {
        Pod el = Pod::element("Fs.List", {{"dir", text("src")}});
        auto files = cx.first_value(el);
        if (!files) return unexpected(Fault::answer("Fs.List unanswered"));
        auto node = prism_rtl::make_rc<Lister>(std::move(path), Hold<Pod>(*files, cx));
        cx.added(node->path.at(0), el);
        cx.added(node->path.at(1), Pod::element("Fs.File", {{"path", text("src/new.prism")}}));
        cx.added(node->path.at(2), Pod::element("Process.Exit", {}));
        cx.listen(el, node);
        cx.on_started(node);
        return node;
    }
    Done started(Cx& cx) override {
        cx.request(path.at(1), std::nullopt, Pod::choice("Write", {text("")}));
        return {};
    }
    Done answer(std::span<const Segment> at, const Pod&, Cx&) override {
        if (at.size() == 1 && at[0].is_pos(1)) return {};
        return no_place(at);
    }
    Done push(const Pod&, const Pod& v, Cx& cx) override {
        files.set(v, cx);
        auto xs = v.if_array();
        if (!xs) return unexpected(Fault::answer("expected [Text]"));
        cx.request(path.at(2), std::nullopt, to_pod(std::uint8_t(xs->size())));
        return {};
    }
};

}  // namespace

TEST(a_write_under_a_listed_directory_pushes_the_new_listing) {
    CliWorld world(mem({{"src/a.prism", ""}, {"src/b.txt", ""}}), {});
    CHECK_EQ(int(drive<Prog<Lister>>(world, {}, 100)), 2);
}

TEST(a_listing_joins_the_directory_as_written) {
    CHECK_EQ(joined("examples", "a.prism"), std::string("examples/a.prism"));
    CHECK_EQ(joined("examples/", "a.prism"), std::string("examples/a.prism"));
    CHECK_EQ(joined(".", "a.prism"), std::string("a.prism"));
    CHECK_EQ(joined("/", "a.prism"), std::string("/a.prism"));
}

TEST(a_memory_listing_takes_only_what_is_under_the_directory) {
    MemFs fs;
    for (const char* p : {"ex/b.prism", "ex/a/c.prismtest", "ex/a/c.expected", "exx/d.prism", "top.prism"}) (void)fs.write(p, "");
    CHECK(fs.list("ex") == (std::vector<std::string>{"ex/a/c.prismtest", "ex/b.prism"}));
    CHECK(fs.list(".") == (std::vector<std::string>{"ex/a/c.prismtest", "ex/b.prism", "exx/d.prism", "top.prism"}));
    CHECK(fs.list("nowhere").empty());
}

// ---- @Later -----------------------------------------------------------------------------------

namespace {

template <int MS>
struct Waker : Node {
    Path path;
    explicit Waker(Path path) : path(std::move(path)) {}
    static Result<prism_rtl::Rc<Waker>> construct(Path path, Cx& cx) {
        auto node = prism_rtl::make_rc<Waker>(std::move(path));
        cx.added(node->path.at(0), Pod::element("Time.After", {}));
        cx.added(node->path.at(1), Pod::element("Process.Exit", {}));
        cx.on_started(node);
        return node;
    }
    Done started(Cx& cx) override {
        Pod wake = Pod::record("Wake", {{"after", Pod::duration(Nanos(MS) * 1000000)}, {"context", text("0")}});
        cx.request(path.at(0), std::nullopt, wake);
        return {};
    }
    Done answer(std::span<const Segment> at, const Pod& v, Cx& cx) override {
        const Pod* c = v.field("context");
        if (at.size() == 1 && at[0].is_pos(0) && c && *c == text("0")) {
            cx.request(path.at(1), std::nullopt, to_pod(std::uint8_t(cx.tick())));
            return {};
        }
        return no_place(at);
    }
};

struct Forever : Node {
    Defer<Unit> defer;
    static Result<prism_rtl::Rc<Forever>> construct(Path, Cx& cx) {
        auto node = prism_rtl::make_rc<Forever>();
        node->defer.arm(Unit{}, node, cx);
        return node;
    }
    Done deferred(Cx& cx) override {
        defer.arm(Unit{}, me(), cx);
        return {};
    }
};

struct Quiet : Node {
    static Result<prism_rtl::Rc<Quiet>> construct(Path, Cx&) { return prism_rtl::make_rc<Quiet>(); }
};

}  // namespace

TEST(at_later_is_answered_one_tick_later) {
    CliWorld world(mem({}), {});
    CHECK_EQ(int(drive<Prog<Waker<0>>>(world, {}, 100)), 2);
}

TEST(a_wake_with_a_delay_waits_for_it) {
    CliWorld world(mem({}), {});
    auto t = std::chrono::steady_clock::now();
    CHECK_EQ(int(drive<Prog<Waker<30>>>(world, {}, 100)), 2);
    CHECK(std::chrono::steady_clock::now() - t >= std::chrono::milliseconds(30));
}

TEST(a_run_that_exhausts_patience_exits_1) {
    CliWorld world(mem({}), {});
    CHECK_EQ(int(drive<Prog<Forever>>(world, {}, 5)), 1);
}

TEST(a_program_with_nothing_left_to_do_exits_0) {
    CliWorld world(mem({}), {});
    CHECK_EQ(int(drive<Prog<Quiet>>(world, {}, 5)), 0);
}

// ---- The prelude's tags (worlds/cli/rust/src/prelude.rs) ---------------------------------------------

TEST(the_subtree_is_the_last_generation_or_key) {
    Path p = Path::root().at(3).child(Segment::gen(0)).at(2);
    CHECK(subtree_of(p) == Path::root().at(3).child(Segment::gen(0)));
    CHECK(subtree_of(Path::root().at(1)) == Path::root());
}

TEST(a_supervisor_hears_the_report_and_the_subtree_goes_quiet) {
    Prelude pre;
    Path sup = Path::root().at(0);
    Path unwrap = Path::root().at(3).child(Segment::gen(0)).at(2);
    Patch patch;
    patch.added.emplace_back(sup, Pod::element("Prelude.Exceptions", {}));
    pre.track(patch);
    pre.unwrap(unwrap, Pod::ok(Pod::int64(5)));
    pre.unwrap(unwrap, Pod::failed(to_pod(Error("Fs.NotFound", Text("gone")))));
    auto answers = pre.take_answers();
    CHECK(answers[0] == std::make_pair(unwrap, Pod::int64(5)));
    CHECK(answers[1].first == sup);
    CHECK(answers[1].second.field("message") && *answers[1].second.field("message") == text("gone"));
    CHECK(pre.is_poisoned(unwrap) && !pre.is_poisoned(sup));
    CHECK(!pre.exit());
}
