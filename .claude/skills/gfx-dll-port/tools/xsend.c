/* Synthetic X events to one window only (XSendEvent), so a scripted input check never types into
 * or clicks anything else on the desktop. Usage: xsend <window-id> <cmd> [args]...
 *   key <keysym-name> [shift]      press+release; `#<n>` names X keycode n itself (`#66`, the
 *                                  Caps Lock key, whatever keysym the layout gives it)
 *   hold <keysym-name> <ms>        press, wait, release
 *   motion <x> <y>                 MotionNotify
 *   button <n> <x> <y>             press+release
 *   down|up <n> <x> <y>            press or release alone (a drag)
 *   focus in|out
 *   delete                         WM_DELETE_WINDOW */
#include <X11/Xlib.h>
#include <X11/Xutil.h>
#include <X11/keysym.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
static Display *d; static Window w;
static void send(XEvent *e, long mask) { e->xany.display = d; e->xany.window = w; XSendEvent(d, w, True, mask, e); XFlush(d); }
static void key(const char *name, unsigned state, int press) {
    XEvent e; memset(&e, 0, sizeof e);
    e.type = press ? KeyPress : KeyRelease;
    e.xkey.root = DefaultRootWindow(d); e.xkey.subwindow = None; e.xkey.time = CurrentTime;
    e.xkey.x = e.xkey.y = e.xkey.x_root = e.xkey.y_root = 1; e.xkey.same_screen = True;
    e.xkey.state = state; e.xkey.keycode = name[0] == '#' ? (unsigned)atoi(name + 1) : XKeysymToKeycode(d, XStringToKeysym(name));
    send(&e, press ? KeyPressMask : KeyReleaseMask);
}
int main(int argc, char **argv) {
    if (argc < 3) return 2;
    d = XOpenDisplay(NULL); if (!d) return 1;
    w = strtoul(argv[1], NULL, 0);
    const char *cmd = argv[2];
    XEvent e; memset(&e, 0, sizeof e);
    if (!strcmp(cmd, "key")) { unsigned st = argc > 4 ? ShiftMask : 0; key(argv[3], st, 1); key(argv[3], st, 0); }
    else if (!strcmp(cmd, "hold")) { key(argv[3], 0, 1); usleep(atoi(argv[4]) * 1000); key(argv[3], 0, 0); }
    else if (!strcmp(cmd, "motion")) { e.type = MotionNotify; e.xmotion.x = atoi(argv[3]); e.xmotion.y = atoi(argv[4]); e.xmotion.same_screen = True; send(&e, PointerMotionMask); }
    else if (!strcmp(cmd, "button")) { int b = atoi(argv[3]);
        for (int p = 1; p >= 0; p--) { memset(&e, 0, sizeof e); e.type = p ? ButtonPress : ButtonRelease; e.xbutton.button = b; e.xbutton.x = atoi(argv[4]); e.xbutton.y = atoi(argv[5]); e.xbutton.same_screen = True; send(&e, p ? ButtonPressMask : ButtonReleaseMask); } }
    else if (!strcmp(cmd, "down") || !strcmp(cmd, "up")) { int p = !strcmp(cmd, "down"); e.type = p ? ButtonPress : ButtonRelease; e.xbutton.button = atoi(argv[3]); e.xbutton.x = atoi(argv[4]); e.xbutton.y = atoi(argv[5]); e.xbutton.same_screen = True; send(&e, p ? ButtonPressMask : ButtonReleaseMask); }
    else if (!strcmp(cmd, "focus")) { e.type = !strcmp(argv[3], "in") ? FocusIn : FocusOut; e.xfocus.mode = NotifyNormal; e.xfocus.detail = NotifyNonlinear; send(&e, FocusChangeMask); }
    else if (!strcmp(cmd, "delete")) { e.type = ClientMessage; e.xclient.message_type = XInternAtom(d, "WM_PROTOCOLS", False); e.xclient.format = 32; e.xclient.data.l[0] = XInternAtom(d, "WM_DELETE_WINDOW", False); e.xclient.data.l[1] = CurrentTime; send(&e, NoEventMask); }
    else return 2;
    XCloseDisplay(d);
    return 0;
}
