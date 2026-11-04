// A robot on an X display, a client of its own: it reads pixels of the root window and
// clicks through XTest. Under Xvfb there is no window manager, so a window sits at the
// origin and the screen's pixels are the window's.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#pragma once

#include <X11/Xlib.h>
#include <X11/Xutil.h>
#include <X11/extensions/XTest.h>

#include <chrono>
#include <cstdint>
#include <cstdio>
#include <string>
#include <thread>

namespace robot {

struct Robot {
    Display* d = XOpenDisplay(nullptr);
    ~Robot() {
        if (d) XCloseDisplay(d);
    }
    explicit operator bool() const { return d != nullptr; }

    /// The pixel at (x, y) of the screen, 0xRRGGBB.
    std::uint32_t pixel(int x, int y) {
        XImage* image = XGetImage(d, DefaultRootWindow(d), x, y, 1, 1, AllPlanes, ZPixmap);
        if (!image) return 0xffffffff;
        unsigned long px = XGetPixel(image, 0, 0);
        XDestroyImage(image);
        return std::uint32_t(px & 0xffffff);
    }

    /// Wait up to `seconds` for (x, y) to be `rgb`.
    bool wait_for(int x, int y, std::uint32_t rgb, double seconds) {
        auto deadline = std::chrono::steady_clock::now() + std::chrono::duration<double>(seconds);
        while (std::chrono::steady_clock::now() < deadline) {
            if (pixel(x, y) == rgb) return true;
            std::this_thread::sleep_for(std::chrono::milliseconds(50));
        }
        std::fprintf(stderr, "robot: (%d, %d) is %06x, not %06x\n", x, y, unsigned(pixel(x, y)), unsigned(rgb));
        return false;
    }

    /// Move to (x, y) and click the left button.
    void click(int x, int y) {
        XTestFakeMotionEvent(d, -1, x, y, 0);
        XTestFakeButtonEvent(d, 1, True, 0);
        XTestFakeButtonEvent(d, 1, False, 0);
        XFlush(d);
    }

    /// Ask the top-level window named `title` to close, as a window manager would
    /// (WM_DELETE_WINDOW).
    bool close(const std::string& title) {
        ::Window root = DefaultRootWindow(d), parent, *kids = nullptr;
        unsigned n = 0;
        if (!XQueryTree(d, root, &root, &parent, &kids, &n)) return false;
        bool sent = false;
        for (unsigned i = 0; i < n && !sent; ++i) {
            char* name = nullptr;
            if (XFetchName(d, kids[i], &name) && name) {
                if (title == name) {
                    XEvent ev{};
                    ev.xclient.type = ClientMessage;
                    ev.xclient.window = kids[i];
                    ev.xclient.message_type = XInternAtom(d, "WM_PROTOCOLS", False);
                    ev.xclient.format = 32;
                    ev.xclient.data.l[0] = long(XInternAtom(d, "WM_DELETE_WINDOW", False));
                    ev.xclient.data.l[1] = CurrentTime;
                    XSendEvent(d, kids[i], False, NoEventMask, &ev);
                    XFlush(d);
                    sent = true;
                }
                XFree(name);
            }
        }
        if (kids) XFree(kids);
        return sent;
    }
};

}  // namespace robot
