// A thread with a 4 GB stack that runs jobs one at a time for the thread that owns the
// windows. The window loop keeps the main thread (AppKit requires it on macOS) and hands
// each tick, and the program's teardown, to this thread, which a deep recursion in the
// program needs: the emitted `main` gives the CLI World the same stack (`on_big_stack`).
// `run` blocks until the job is done, so the two threads never run at once and the
// values they share pass through the mutex, never concurrently.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#pragma once

#include <pthread.h>

#include <condition_variable>
#include <functional>
#include <mutex>

namespace prism_world_canvas {

class BigStack {
public:
    BigStack();
    ~BigStack();
    BigStack(const BigStack&) = delete;
    BigStack& operator=(const BigStack&) = delete;

    /// Runs `job` on the big stack and waits for it; on the caller's own stack when no
    /// such thread could be made (said once on standard error when it was made).
    void run(const std::function<void()>& job);

private:
    static void* main(void* self);

    std::mutex mutex_;
    std::condition_variable cv_;
    const std::function<void()>* job_ = nullptr;
    bool done_ = false;
    bool quit_ = false;
    bool started_ = false;
    pthread_t thread_{};
};

}  // namespace prism_world_canvas
