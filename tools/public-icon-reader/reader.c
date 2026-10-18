/* Read-only, one-shot Root-admin public-app icon reader. No service or fallback.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
#define _DARWIN_C_SOURCE 1
#define _POSIX_C_SOURCE 200809L
#include <errno.h>
#include <fcntl.h>
#include <inttypes.h>
#include <poll.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <time.h>
#include <unistd.h>

#define ICON_CAP (1024U * 1024U)
#define REL_CAP 512U
#define COMPONENT_CAP 128U
#define REL_COMPONENTS 8U
#define NODE_CAP 24U
#define HEADER_CAP 4096U
#define IO_SECONDS 5
#ifdef __APPLE__
#define MT(s) ((s).st_mtimespec)
#define CT(s) ((s).st_ctimespec)
#else
#define MT(s) ((s).st_mtim)
#define CT(s) ((s).st_ctim)
#endif

struct node { int fd, parent; const char *name; struct stat before; };
struct chain { struct node nodes[NODE_CAP]; size_t count; uid_t uid; gid_t gid; };
#ifdef PIR_TESTING
/* Compile-time test translation unit only; no production argv/environment seam. */
static void (*test_hook)(int point);
#define HOOK(point) do { if (test_hook) test_hook(point); } while (0)
#else
#define HOOK(point) ((void)0)
#endif

static int same_stat(const struct stat *a, const struct stat *b) {
    return a->st_dev == b->st_dev && a->st_ino == b->st_ino &&
        a->st_mode == b->st_mode && a->st_uid == b->st_uid && a->st_gid == b->st_gid &&
        a->st_nlink == b->st_nlink && a->st_size == b->st_size &&
        MT(*a).tv_sec == MT(*b).tv_sec && MT(*a).tv_nsec == MT(*b).tv_nsec &&
        CT(*a).tv_sec == CT(*b).tv_sec && CT(*a).tv_nsec == CT(*b).tv_nsec;
}
static int plain_owned(const struct stat *s, uid_t uid, gid_t gid, int directory) {
    return s->st_uid == uid && s->st_gid == gid && !(s->st_mode & 07000) &&
        (directory ? S_ISDIR(s->st_mode) && s->st_nlink > 0 && (s->st_mode & S_IXOTH) :
            S_ISREG(s->st_mode) && (s->st_mode & S_IROTH) && s->st_nlink == 1 && s->st_size > 0 &&
            (uintmax_t)s->st_size <= ICON_CAP);
}
static int check_chain(const struct chain *c) {
    for (size_t i = 0; i < c->count; ++i) {
        struct stat held, named;
        const struct node *n = &c->nodes[i];
        if (fstat(n->fd, &held) ||
            fstatat(n->parent, n->name, &named, AT_SYMLINK_NOFOLLOW) ||
            !plain_owned(&held, c->uid, c->gid, 1) ||
            !same_stat(&n->before, &held) || !same_stat(&held, &named)) return -1;
    }
    return 0;
}
static void close_chain(struct chain *c) {
    while (c->count) close(c->nodes[--c->count].fd);
}
static int anchor(struct chain *c, const char *name, uid_t uid, gid_t gid) {
    struct stat named, held;
    int fd;
    memset(c, 0, sizeof(*c)); c->uid = uid; c->gid = gid;
    if (fstatat(AT_FDCWD, name, &named, AT_SYMLINK_NOFOLLOW) ||
        !plain_owned(&named, uid, gid, 1)) return -1;
    fd = open(name, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if (fd < 0) return -1;
    if (fstat(fd, &held) || !same_stat(&named, &held)) { close(fd); return -1; }
    c->nodes[c->count++] = (struct node){fd, AT_FDCWD, name, held};
    return check_chain(c);
}
static int descend(struct chain *c, const char *name) {
    struct stat named, held;
    int fd, parent = c->nodes[c->count - 1].fd;
    if (c->count >= NODE_CAP || check_chain(c) ||
        fstatat(parent, name, &named, AT_SYMLINK_NOFOLLOW) ||
        !plain_owned(&named, c->uid, c->gid, 1)) return -1;
    fd = openat(parent, name, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if (fd < 0) return -1;
    if (fstat(fd, &held) || !same_stat(&named, &held)) { close(fd); return -1; }
    c->nodes[c->count++] = (struct node){fd, parent, name, held};
    return check_chain(c);
}
static int app_id(const char *id) {
    size_t n = strnlen(id, 181);
    if (!n || n > 180 || !((id[0] >= 'A' && id[0] <= 'Z') ||
        (id[0] >= 'a' && id[0] <= 'z') || (id[0] >= '0' && id[0] <= '9'))) return 0;
    for (size_t i = 0; i < n; ++i) {
        unsigned char x = (unsigned char)id[i];
        if (!((x >= 'a' && x <= 'z') || (x >= 'A' && x <= 'Z') ||
            (x >= '0' && x <= '9') || x == '.' || x == '_' || x == '-')) return 0;
    }
    return 1;
}
/* ASCII printable component names only; no URI, absolute, dot or empty segments. */
static int relative_parts(const char *relative, char *copy, char **parts, size_t *count) {
    size_t n = strnlen(relative, REL_CAP + 1), start = 0;
    *count = 0;
    if (!n || n > REL_CAP) return -1;
    memcpy(copy, relative, n + 1);
    for (size_t i = 0; i <= n; ++i) {
        unsigned char x = (unsigned char)relative[i];
        if (i < n && (x < 32 || x > 126 || x == '\\' || x == ':')) return -1;
        if (i == n || x == '/') {
            size_t len = i - start;
            if (!len || len > COMPONENT_CAP || *count >= REL_COMPONENTS ||
                (len == 1 && relative[start] == '.') ||
                (len == 2 && relative[start] == '.' && relative[start + 1] == '.')) return -1;
            copy[i] = 0; parts[(*count)++] = copy + start; start = i + 1;
        }
    }
    return 0;
}
static int extension(const char *name) {
    const char *dot = strrchr(name, '.'); char ext[6]; size_t n;
    if (!dot || (n = strlen(dot)) >= sizeof(ext)) return 0;
    for (size_t i = 0; i <= n; ++i) {
        unsigned char x = (unsigned char)dot[i];
        ext[i] = (char)((x >= 'A' && x <= 'Z') ? x + ('a' - 'A') : x);
    }
    return !strcmp(ext, ".png") ? 1 : (!strcmp(ext, ".jpg") || !strcmp(ext, ".jpeg")) ? 2 : 0;
}
static int signature(const unsigned char *body, size_t n, int kind) {
    static const unsigned char png[8] = {137,80,78,71,13,10,26,10};
    return kind == 1 ? n >= 8 && !memcmp(body, png, 8) :
        n >= 3 && body[0] == 255 && body[1] == 216 && body[2] == 255;
}
static int json_text(char *out, size_t cap, const char *in) {
    size_t n = 0;
    for (size_t i = 0; in[i]; ++i) {
        if (in[i] == '"' || in[i] == '\\') { if (n + 2 >= cap) return -1; out[n++] = '\\'; }
        if (n + 1 >= cap) return -1;
        out[n++] = in[i];
    }
    out[n] = 0; return 0;
}
static int write_all(int fd, const void *data, size_t n) {
    const unsigned char *p = data;
    while (n) {
        struct pollfd pollfd = {fd, POLLOUT, 0};
        if (poll(&pollfd, 1, IO_SECONDS * 1000) != 1 || !(pollfd.revents & POLLOUT)) return -1;
        ssize_t written = write(fd, p, n > 4096 ? 4096 : n);
        if (written <= 0) return -1;
        p += (size_t)written; n -= (size_t)written;
    }
    return 0;
}
/* chain owns the already validated literal public root. Remains held to emission. */
static int read_emit(struct chain *c, const char *root_kind, const char *id,
                     const char *relative, int output) {
    char names[REL_CAP + 1], *parts[REL_COMPONENTS], escaped[2 * REL_CAP + 1];
    size_t count, size = 0; int fd = -1, result = -1, kind;
    struct stat path_before, held, after, named;
    unsigned char *body = NULL, extra, prefix[8] = {'L','P','I','1',0,0,0,0};
    char header[HEADER_CAP]; int header_size;
    if (!app_id(id) || relative_parts(relative, names, parts, &count) ||
        !(kind = extension(parts[count - 1])) || descend(c, id)) goto done;
    HOOK(1);
    for (size_t i = 0; i + 1 < count; ++i) if (descend(c, parts[i])) goto done;
    int parent = c->nodes[c->count - 1].fd;
    if (check_chain(c) || fstatat(parent, parts[count - 1], &path_before, AT_SYMLINK_NOFOLLOW) ||
        !plain_owned(&path_before, c->uid, c->gid, 0)) goto done;
    HOOK(5);
    fd = openat(parent, parts[count - 1], O_RDONLY | O_NONBLOCK | O_NOFOLLOW | O_CLOEXEC);
    if (fd < 0 || fstat(fd, &held) || !same_stat(&held, &path_before)) goto done;
    HOOK(2);
    size = (size_t)held.st_size; body = malloc(size);
    if (!body) goto done;
    for (size_t used = 0; used < size;) {
        ssize_t n = read(fd, body + used, size - used);
        if (n <= 0) goto done;
        used += (size_t)n;
    }
    if (read(fd, &extra, 1) != 0) goto done;
    HOOK(3);
    if (!signature(body, size, kind) || json_text(escaped, sizeof(escaped), relative)) goto done;
    header_size = snprintf(header, sizeof(header),
        "{\"schema\":1,\"root_kind\":\"%s\",\"app_id\":\"%s\",\"relative\":\"%s\","
        "\"bytes\":%zu,\"device\":%" PRIuMAX ",\"inode\":%" PRIuMAX ",\"uid\":%" PRIuMAX ","
        "\"gid\":%" PRIuMAX ",\"mode\":%" PRIuMAX ",\"nlink\":%" PRIuMAX ","
        "\"mtime_seconds\":%" PRIdMAX ",\"mtime_nanos\":%ld,\"ctime_seconds\":%" PRIdMAX ",\"ctime_nanos\":%ld}",
        root_kind, id, escaped, size, (uintmax_t)held.st_dev, (uintmax_t)held.st_ino,
        (uintmax_t)held.st_uid, (uintmax_t)held.st_gid, (uintmax_t)held.st_mode,
        (uintmax_t)held.st_nlink, (intmax_t)MT(held).tv_sec, MT(held).tv_nsec,
        (intmax_t)CT(held).tv_sec, CT(held).tv_nsec);
    if (header_size <= 0 || (size_t)header_size >= sizeof(header)) goto done;
    HOOK(4);
    /* Nothing has reached stdout. Revalidate every held/named chain and file epoch. */
    if (check_chain(c) || fstat(fd, &after) ||
        fstatat(parent, parts[count - 1], &named, AT_SYMLINK_NOFOLLOW) ||
        !plain_owned(&after, c->uid, c->gid, 0) || !same_stat(&held, &after) ||
        !same_stat(&held, &named)) goto done;
    prefix[4] = (unsigned char)((unsigned)header_size >> 24);
    prefix[5] = (unsigned char)((unsigned)header_size >> 16);
    prefix[6] = (unsigned char)((unsigned)header_size >> 8);
    prefix[7] = (unsigned char)header_size;
    if (write_all(output, prefix, sizeof(prefix)) ||
        write_all(output, header, (size_t)header_size) || write_all(output, body, size)) goto done;
    result = 0;
 done:
    if (fd >= 0) close(fd);
    free(body); return result;
}
static void timed_out(int signal_number) { (void)signal_number; _exit(124); }
static int start_deadline(unsigned seconds) {
    struct sigaction action; sigset_t unblocked;
    memset(&action, 0, sizeof(action)); action.sa_handler = timed_out;
    if (sigemptyset(&action.sa_mask) || sigemptyset(&unblocked) ||
        sigaddset(&unblocked, SIGALRM) || sigaction(SIGALRM, &action, NULL) ||
        sigprocmask(SIG_UNBLOCK, &unblocked, NULL)) return -1;
    alarm(seconds); return 0;
}
/* One reserved lifecycle write. No retry, path I/O or stdout on write refusal. */
static int emit_reader_pid(int fd) {
    char line[96]; pid_t pid = getpid();
    if (pid <= 1) return -1;
    int n = snprintf(line, sizeof(line), "LUMA_PUBLIC_ICON_READER_V1 pid=%" PRIdMAX "\n", (intmax_t)pid);
    if (n <= 0 || (size_t)n >= sizeof(line)) return -1;
    return write(fd, line, (size_t)n) == n ? 0 : -1;
}
/* Fixed literal arguments in production; fixture anchor/UID only in test TU. */
static int request_from(const char *anchor_path, const char *public_root,
                        uid_t uid, gid_t gid, const char *kind, const char *id,
                        const char *relative, int output, int lifecycle) {
    struct chain c; char root_copy[128]; char *part, *state;
    int result = -1;
    if (emit_reader_pid(lifecycle)) return -2;
    HOOK(6); /* Test-only evidence that no path walk starts after failed PID write. */
    if (anchor(&c, anchor_path, uid, gid)) { close_chain(&c); return -1; }
    size_t length = strnlen(public_root, sizeof(root_copy));
    if (length >= sizeof(root_copy)) goto closed;
    memcpy(root_copy, public_root, length + 1);
    for (part = strtok_r(root_copy, "/", &state); part; part = strtok_r(NULL, "/", &state))
        if (descend(&c, part)) goto closed;
    result = read_emit(&c, kind, id, relative, output);
 closed:
    close_chain(&c); return result;
}
#ifndef PIR_TESTING
int main(int argc, char **argv) {
    static const struct { const char *kind, *root; } roots[] = {
        {"lg_system", "/usr/palm/applications"},
        {"encrypted_apps", "/media/cryptofs/apps/usr/palm/applications"},
        {"developer_apps", "/media/developer/apps/usr/palm/applications"}
    };
    int root = -1, result = -1;
    if (start_deadline(IO_SECONDS)) return 1; /* Original deadline; never renewed. */
    if (argc != 4 || getuid() != 0 || geteuid() != 0 || getgid() != 0 || getegid() != 0) goto refused;
    for (size_t i = 0; i < sizeof(roots) / sizeof(roots[0]); ++i)
        if (!strcmp(argv[1], roots[i].kind)) root = (int)i;
    if (root < 0) goto refused;
    result = request_from("/", roots[root].root, 0, 0, roots[root].kind,
                          argv[2], argv[3], STDOUT_FILENO, STDERR_FILENO);
    if (!result) return 0;
    if (result == -2) return 1; /* Missing/partial lifecycle record is not retried. */
 refused:
    fputs("public icon read refused\n", stderr); return 1;
}
#endif
