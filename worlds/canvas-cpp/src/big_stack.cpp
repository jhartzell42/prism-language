// The big-stack thread (big_stack.hpp).
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include "prism_world_canvas/big_stack.hpp"

#include <cstdio>

namespace prism_world_canvas {

BigStack::BigStack() {
    pthread_attr_t attr;
    pthread_attr_init(&attr);
    pthread_attr_setstacksize(&attr, std::size_t(4) << 30);
    started_ = pthread_create(&thread_, &attr, &BigStack::main, this) == 0;
    pthread_attr_destroy(&attr);
    if (!started_) std::fprintf(stderr, "prism canvas: no thread with a 4 GB stack: ticking on the window thread's\n");
}

BigStack::~BigStack() {
    if (!started_) return;
    {
        std::lock_guard lock(mutex_);
        quit_ = true;
    }
    cv_.notify_all();
    pthread_join(thread_, nullptr);
}

void BigStack::run(const std::function<void()>& job) {
    if (!started_) return job();
    std::unique_lock lock(mutex_);
    job_ = &job;
    done_ = false;
    cv_.notify_all();
    cv_.wait(lock, [&] { return done_; });
}

void* BigStack::main(void* p) {
    auto* self = static_cast<BigStack*>(p);
    std::unique_lock lock(self->mutex_);
    for (;;) {
        self->cv_.wait(lock, [&] { return self->job_ || self->quit_; });
        if (self->job_) {
            (*self->job_)();
            self->job_ = nullptr;
            self->done_ = true;
            self->cv_.notify_all();
            continue;
        }
        return nullptr;
    }
}

}  // namespace prism_world_canvas
