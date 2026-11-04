/* The GUI benchmark's table in C with GTK 3 (bench/gui/README.md): the same rows, labels,
 * buttons and operations as bench/prism/bench.prism, and its own driver. A step clicks its
 * button with gtk_button_clicked; a timed step's clock starts just before the click and
 * stops at the window's next frame-clock "after-paint" once the click's handler has run,
 * followed by gdk_display_sync, so the frame is with the window system.
 *
 *   table OP RUNS WARMUP     prints `OP RUN MS` per timed run, then `peak_rss_bytes N`
 *
 * Written by Claude on the claude-self-hosting branch; subject to change. */

#include <gtk/gtk.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <time.h>

/* ---- The table ------------------------------------------------------------------------ */

typedef struct {
    unsigned id;
    char *label;
    GtkWidget *box;   /* the row: id label, label button, remove button */
    GtkWidget *name;  /* the label button */
    GtkWidget *x;     /* the remove button */
} Row;

static GPtrArray *rows;     /* of Row*, in order */
static GtkWidget *list;     /* the column the rows are in */
static Row *selected;
static unsigned next_id = 1;
static unsigned long seed = 42;

static const char *adjectives[] = {"pretty", "large", "big", "small", "tall", "short", "long", "handsome", "plain",
                                   "quaint", "clean", "elegant", "easy", "angry", "crazy", "helpful", "mushy", "odd",
                                   "unsightly", "adorable", "important", "inexpensive", "cheap", "expensive", "fancy"};
static const char *colours[] = {"red", "yellow", "blue", "green", "pink", "brown", "purple", "brown", "white", "black", "orange"};
static const char *nouns[] = {"table", "chair", "house", "bbq", "desk", "car", "pony", "cookie", "sandwich", "burger",
                              "pizza", "mouse", "keyboard"};
#define COUNT(a) (sizeof(a) / sizeof((a)[0]))

/* bench.prism's generator, so both make the same labels */
static unsigned long next_seed(unsigned long s) { return (s * 1103515245UL + 12345UL) % 2147483648UL; }
static const char *pick(const char **words, size_t n, unsigned long s) { return words[(s / 65536) % n]; }

static void show_label(Row *r) {
    if (r == selected) {
        char *marked = g_strdup_printf("▶ %s", r->label);
        gtk_button_set_label(GTK_BUTTON(r->name), marked);
        g_free(marked);
    } else {
        gtk_button_set_label(GTK_BUTTON(r->name), r->label);
    }
}

static void on_select(GtkButton *b, gpointer data);
static void on_remove(GtkButton *b, gpointer data);

static Row *add_row(void) {
    Row *r = g_new0(Row, 1);
    unsigned long s1 = next_seed(seed), s2 = next_seed(s1), s3 = next_seed(s2);
    seed = s3;
    r->id = next_id++;
    r->label = g_strdup_printf("%s %s %s", pick(adjectives, COUNT(adjectives), s1), pick(colours, COUNT(colours), s2),
                               pick(nouns, COUNT(nouns), s3));
    r->box = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 4);
    char id[16];
    snprintf(id, sizeof id, "%u", r->id);
    gtk_box_pack_start(GTK_BOX(r->box), gtk_label_new(id), FALSE, FALSE, 0);
    r->name = gtk_button_new_with_label(r->label);
    r->x = gtk_button_new_with_label("x");
    g_signal_connect(r->name, "clicked", G_CALLBACK(on_select), r);
    g_signal_connect(r->x, "clicked", G_CALLBACK(on_remove), r);
    gtk_box_pack_start(GTK_BOX(r->box), r->name, FALSE, FALSE, 0);
    gtk_box_pack_start(GTK_BOX(r->box), r->x, FALSE, FALSE, 0);
    gtk_box_pack_start(GTK_BOX(list), r->box, FALSE, FALSE, 0);
    gtk_widget_show_all(r->box);
    g_ptr_array_add(rows, r);
    return r;
}

static void free_row(Row *r) {
    if (r == selected) selected = NULL;
    gtk_widget_destroy(r->box);
    g_free(r->label);
    g_free(r);
}

static void clear_rows(void) {
    for (guint i = 0; i < rows->len; ++i) free_row(g_ptr_array_index(rows, i));
    g_ptr_array_set_size(rows, 0);
}

static void on_create(GtkButton *b, gpointer n) {
    (void)b;
    clear_rows();
    for (int i = 0; i < GPOINTER_TO_INT(n); ++i) add_row();
}

static void on_append(GtkButton *b, gpointer n) {
    (void)b;
    for (int i = 0; i < GPOINTER_TO_INT(n); ++i) add_row();
}

static void on_update(GtkButton *b, gpointer d) {
    (void)b, (void)d;
    for (guint i = 0; i < rows->len; i += 10) {
        Row *r = g_ptr_array_index(rows, i);
        char *bang = g_strdup_printf("%s !!!", r->label);
        g_free(r->label);
        r->label = bang;
        show_label(r);
    }
}

static void on_clear(GtkButton *b, gpointer d) {
    (void)b, (void)d;
    clear_rows();
}

static void on_swap(GtkButton *b, gpointer d) {
    (void)b, (void)d;
    if (rows->len < 999) return;
    Row *a = g_ptr_array_index(rows, 1), *z = g_ptr_array_index(rows, 998);
    rows->pdata[1] = z;
    rows->pdata[998] = a;
    gtk_box_reorder_child(GTK_BOX(list), z->box, 1);
    gtk_box_reorder_child(GTK_BOX(list), a->box, 998);
}

static void on_select(GtkButton *b, gpointer data) {
    (void)b;
    Row *old = selected;
    selected = data;
    if (old) show_label(old);
    show_label(selected);
}

static void on_remove(GtkButton *b, gpointer data) {
    (void)b;
    g_ptr_array_remove(rows, data);
    free_row(data);
}

/* ---- The driver ------------------------------------------------------------------------- */

typedef enum { CREATE_1K, CREATE_10K, APPEND_1K, UPDATE, CLEAR, SWAP, ROW_LABEL, ROW_X } Target;
typedef struct {
    Target target;
    gboolean timed;
} Step;

static GtkWidget *buttons[6];
static Step *steps;
static int nsteps, next_step, run;
static gboolean waiting, timing;
static double t0;
static const char *op;
static GtkWidget *window;

static double now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1e3 + ts.tv_nsec / 1e6;
}

static long peak_rss_bytes(void) {
    struct rusage u;
    getrusage(RUSAGE_SELF, &u);
#ifdef __APPLE__
    return (long)u.ru_maxrss;
#else
    return (long)u.ru_maxrss * 1024;
#endif
}

static gboolean step(gpointer d);

static void after_paint(GdkFrameClock *clock, gpointer d) {
    (void)clock, (void)d;
    if (!waiting) return;
    gdk_display_sync(gtk_widget_get_display(window));
    if (timing) {
        printf("%s %d %.3f\n", op, ++run, now_ms() - t0);
        fflush(stdout);
    }
    waiting = FALSE;
    g_idle_add(step, NULL);
}

static gboolean step(gpointer d) {
    (void)d;
    if (next_step == nsteps) {
        printf("peak_rss_bytes %ld\n", peak_rss_bytes());
        gtk_main_quit();
        return G_SOURCE_REMOVE;
    }
    Step s = steps[next_step++];
    GtkWidget *b = NULL;
    if (s.target <= SWAP) b = buttons[s.target];
    else if (rows->len >= 2) {
        Row *r = g_ptr_array_index(rows, 1);
        b = s.target == ROW_LABEL ? r->name : r->x;
    }
    if (!b) {
        fprintf(stderr, "table: no button for step %d\n", next_step);
        exit(1);
    }
    waiting = TRUE;
    timing = s.timed;
    t0 = now_ms();
    gtk_button_clicked(GTK_BUTTON(b));
    gtk_widget_queue_draw(window); /* a click that changed nothing still ends at a frame */
    return G_SOURCE_REMOVE;
}

static void script(const char *name, int runs, int warmup) {
    static const struct {
        const char *name;
        Step steps[3];
        int n;
    } all[] = {
        {"create1k", {{CREATE_1K, TRUE}, {CLEAR, FALSE}}, 2},
        {"replace1k", {{CREATE_1K, FALSE}, {CREATE_1K, TRUE}, {CLEAR, FALSE}}, 3},
        {"update10th", {{CREATE_1K, FALSE}, {UPDATE, TRUE}, {CLEAR, FALSE}}, 3},
        {"select", {{CREATE_1K, FALSE}, {ROW_LABEL, TRUE}, {CLEAR, FALSE}}, 3},
        {"swap", {{CREATE_1K, FALSE}, {SWAP, TRUE}, {CLEAR, FALSE}}, 3},
        {"remove", {{CREATE_1K, FALSE}, {ROW_X, TRUE}, {CLEAR, FALSE}}, 3},
        {"create10k", {{CREATE_10K, TRUE}, {CLEAR, FALSE}}, 2},
        {"append1k", {{CREATE_1K, FALSE}, {APPEND_1K, TRUE}, {CLEAR, FALSE}}, 3},
        {"clear", {{CREATE_1K, FALSE}, {CLEAR, TRUE}}, 2},
    };
    for (size_t i = 0; i < COUNT(all); ++i) {
        if (strcmp(all[i].name, name) != 0) continue;
        nsteps = (runs + warmup) * all[i].n;
        steps = g_new0(Step, nsteps);
        for (int r = 0, k = 0; r < runs + warmup; ++r)
            for (int j = 0; j < all[i].n; ++j, ++k) {
                steps[k] = all[i].steps[j];
                if (r < warmup) steps[k].timed = FALSE;
            }
        return;
    }
    fprintf(stderr, "table: unknown operation %s\n", name);
    exit(2);
}

static gboolean first_frame(GtkWidget *w, GdkFrameClock *clock, gpointer d) {
    (void)w, (void)d;
    g_signal_connect(clock, "after-paint", G_CALLBACK(after_paint), NULL);
    g_idle_add(step, NULL);
    return G_SOURCE_REMOVE;
}

int main(int argc, char **argv) {
    gtk_init(&argc, &argv);
    if (argc != 4 || atoi(argv[2]) <= 0 || atoi(argv[3]) < 0) {
        fprintf(stderr, "usage: table OP RUNS WARMUP\n");
        return 2;
    }
    op = argv[1];
    script(op, atoi(argv[2]), atoi(argv[3]));
    rows = g_ptr_array_new();

    window = gtk_window_new(GTK_WINDOW_TOPLEVEL);
    gtk_window_set_title(GTK_WINDOW(window), "GTK table");
    gtk_window_set_default_size(GTK_WINDOW(window), 900, 800);
    g_signal_connect(window, "destroy", G_CALLBACK(gtk_main_quit), NULL);
    GtkWidget *column = gtk_box_new(GTK_ORIENTATION_VERTICAL, 2);
    GtkWidget *top = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 4);
    static const char *labels[] = {"Create 1,000 rows", "Create 10,000 rows", "Append 1,000 rows", "Update every 10th row",
                                   "Clear", "Swap rows"};
    GCallback handlers[] = {G_CALLBACK(on_create), G_CALLBACK(on_create), G_CALLBACK(on_append),
                            G_CALLBACK(on_update), G_CALLBACK(on_clear), G_CALLBACK(on_swap)};
    int args[] = {1000, 10000, 1000, 0, 0, 0};
    for (int i = 0; i < 6; ++i) {
        buttons[i] = gtk_button_new_with_label(labels[i]);
        g_signal_connect(buttons[i], "clicked", handlers[i], GINT_TO_POINTER(args[i]));
        gtk_box_pack_start(GTK_BOX(top), buttons[i], FALSE, FALSE, 0);
    }
    list = gtk_box_new(GTK_ORIENTATION_VERTICAL, 2);
    GtkWidget *scroll = gtk_scrolled_window_new(NULL, NULL);
    gtk_container_add(GTK_CONTAINER(scroll), list);
    gtk_box_pack_start(GTK_BOX(column), top, FALSE, FALSE, 0);
    gtk_box_pack_start(GTK_BOX(column), scroll, TRUE, TRUE, 0);
    gtk_container_add(GTK_CONTAINER(window), column);
    gtk_widget_show_all(window);
    gtk_widget_add_tick_callback(window, first_frame, NULL, NULL);
    gtk_main();
    return 0;
}
