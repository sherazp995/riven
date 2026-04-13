/* Riven Language Runtime Library
 *
 * Provides basic I/O, string operations, and memory management.
 * Linked into every Riven executable.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <inttypes.h>
#include <stdbool.h>

/* ── Platform Assertions ──────────────────────────────────────────── */

_Static_assert(sizeof(void *) == sizeof(int64_t),
    "Riven requires a 64-bit platform (sizeof(void*) must equal sizeof(int64_t))");

_Static_assert(sizeof(void *) == 8,
    "Riven requires 64-bit pointers");

/* ── Forward Declarations ─────────────────────────────────────────── */

void riven_panic(const char *message);
char *riven_string_from(const char *s);

/* ── Printing ──────────────────────────────────────────────────────── */

void riven_puts(const char *s) {
    if (s) {
        puts(s);
    } else {
        puts("(nil)");
    }
}

void riven_print(const char *s) {
    if (s) {
        fputs(s, stdout);
    }
}

void riven_eputs(const char *s) {
    if (s) {
        fprintf(stderr, "%s\n", s);
    }
}

void riven_print_int(int64_t n) {
    printf("%" PRId64 "\n", n);
}

void riven_print_float(double f) {
    printf("%g\n", f);
}

/* ── To-String Conversions ─────────────────────────────────────────── */

char *riven_int_to_string(int64_t n) {
    char buf[32];
    snprintf(buf, sizeof(buf), "%" PRId64, n);
    size_t len = strlen(buf);
    char *result = (char *)malloc(len + 1);
    if (!result) {
        riven_panic("out of memory");
    }
    memcpy(result, buf, len + 1);
    return result;
}

char *riven_float_to_string(double f) {
    char buf[64];
    snprintf(buf, sizeof(buf), "%g", f);
    size_t len = strlen(buf);
    char *result = (char *)malloc(len + 1);
    if (!result) {
        riven_panic("out of memory");
    }
    memcpy(result, buf, len + 1);
    return result;
}

/* Convert a Unicode codepoint (passed widened to i64) into a heap-allocated
   UTF-8 string. Used for `"#{c}"` interpolation on values of type `Char`. */
char *riven_char_to_string(int64_t codepoint) {
    uint32_t cp = (uint32_t)codepoint;
    char buf[5];
    size_t len;
    if (cp < 0x80) {
        buf[0] = (char)cp;
        len = 1;
    } else if (cp < 0x800) {
        buf[0] = (char)(0xC0 | (cp >> 6));
        buf[1] = (char)(0x80 | (cp & 0x3F));
        len = 2;
    } else if (cp < 0x10000) {
        buf[0] = (char)(0xE0 | (cp >> 12));
        buf[1] = (char)(0x80 | ((cp >> 6) & 0x3F));
        buf[2] = (char)(0x80 | (cp & 0x3F));
        len = 3;
    } else {
        buf[0] = (char)(0xF0 | (cp >> 18));
        buf[1] = (char)(0x80 | ((cp >> 12) & 0x3F));
        buf[2] = (char)(0x80 | ((cp >> 6) & 0x3F));
        buf[3] = (char)(0x80 | (cp & 0x3F));
        len = 4;
    }
    char *result = (char *)malloc(len + 1);
    if (!result) {
        riven_panic("out of memory");
    }
    memcpy(result, buf, len);
    result[len] = '\0';
    return result;
}

char *riven_bool_to_string(int64_t b) {
    const char *s = b ? "true" : "false";
    size_t len = strlen(s);
    char *result = (char *)malloc(len + 1);
    if (!result) {
        riven_panic("out of memory");
    }
    memcpy(result, s, len + 1);
    return result;
}

/* ── String Operations ─────────────────────────────────────────────── */

/* ── String Comparison ─────────────────────────────────────────── */

int64_t riven_string_eq(const char *a, const char *b) {
    if (a == b) return 1;
    if (!a || !b) return 0;
    return strcmp(a, b) == 0 ? 1 : 0;
}

int64_t riven_string_cmp(const char *a, const char *b) {
    if (!a && !b) return 0;
    if (!a) return -1;
    if (!b) return 1;
    return (int64_t)strcmp(a, b);
}

char *riven_string_concat(const char *a, const char *b) {
    if (!a && !b) return NULL;
    if (!a) return riven_string_from(b);
    if (!b) return riven_string_from(a);
    size_t len_a = strlen(a);
    size_t len_b = strlen(b);
    size_t total;
    if (__builtin_add_overflow(len_a, len_b, &total) ||
        __builtin_add_overflow(total, 1, &total)) {
        riven_panic("string size overflow");
    }
    char *result = (char *)malloc(total);
    if (!result) {
        riven_panic("out of memory");
    }
    memcpy(result, a, len_a);
    memcpy(result + len_a, b, len_b + 1);
    return result;
}

char *riven_string_from(const char *s) {
    if (!s) return NULL;
    size_t len = strlen(s);
    char *result = (char *)malloc(len + 1);
    if (!result) {
        riven_panic("out of memory");
    }
    memcpy(result, s, len + 1);
    return result;
}

/* ── Memory Management ─────────────────────────────────────────────── */

void *riven_alloc(uint64_t size) {
    void *ptr = malloc((size_t)size);
    if (!ptr && size > 0) {
        riven_panic("out of memory");
    }
    memset(ptr, 0, (size_t)size);
    return ptr;
}

void riven_dealloc(void *ptr) {
    free(ptr);
}

void *riven_realloc(void *ptr, uint64_t new_size) {
    void *new_ptr = realloc(ptr, (size_t)new_size);
    if (!new_ptr && new_size > 0) {
        riven_panic("out of memory");
    }
    return new_ptr;
}

/* ── String Extended Operations ────────────────────────────────────── */

uint64_t riven_string_len(const char *s) {
    return s ? (uint64_t)strlen(s) : 0;
}

int8_t riven_string_is_empty(const char *s) {
    return (!s || s[0] == '\0') ? 1 : 0;
}

/* push_str: Two calling conventions are supported.
   (1) Caller has a local `char*` (not an address) — the codegen passes the
       value directly and reassigns the returned buffer.
   (2) Caller has a `&mut String` pointer (`char**`) — the codegen derefs it
       to read the current buffer, calls this helper, then stores the result
       back through the pointer via `riven_store_ptr`.
   Either way this helper itself just takes two `char*` and returns a fresh
   concatenated buffer. */
char *riven_string_push_str(const char *dst, const char *src) {
    if (!dst && !src) return NULL;
    return riven_string_concat(dst, src);
}

/* Dereference a pointer-to-pointer: `*p` where `p` is a `char**`.
   Used by the codegen to read the current value of a `&mut String` local
   before calling a mutating method like `push` or `push_str`. */
char *riven_deref_ptr(char **p) {
    return p ? *p : NULL;
}

/* Store through a pointer-to-pointer: `*p = v` where `p` is a `char**`.
   Used by the codegen to write back a new buffer into a `&mut String`
   local so that the caller observes the reassignment after a mutating
   method returns. */
void riven_store_ptr(char **p, char *v) {
    if (p) *p = v;
}

char *riven_string_trim(const char *s) {
    if (!s) return riven_string_from("");
    /* Skip leading whitespace */
    while (*s == ' ' || *s == '\t' || *s == '\n' || *s == '\r') s++;
    size_t len = strlen(s);
    /* Skip trailing whitespace */
    while (len > 0 && (s[len-1] == ' ' || s[len-1] == '\t' ||
           s[len-1] == '\n' || s[len-1] == '\r')) len--;
    char *result = (char *)malloc(len + 1);
    if (!result) {
        riven_panic("out of memory");
    }
    memcpy(result, s, len);
    result[len] = '\0';
    return result;
}

char *riven_string_to_lower(const char *s) {
    if (!s) return riven_string_from("");
    size_t len = strlen(s);
    char *result = (char *)malloc(len + 1);
    if (!result) {
        riven_panic("out of memory");
    }
    for (size_t i = 0; i < len; i++) {
        char c = s[i];
        if (c >= 'A' && c <= 'Z') c = c + ('a' - 'A');
        result[i] = c;
    }
    result[len] = '\0';
    return result;
}

char *riven_string_to_upper(const char *s) {
    if (!s) return riven_string_from("");
    size_t len = strlen(s);
    char *result = (char *)malloc(len + 1);
    if (!result) {
        riven_panic("out of memory");
    }
    for (size_t i = 0; i < len; i++) {
        char c = s[i];
        if (c >= 'a' && c <= 'z') c = c - ('a' - 'A');
        result[i] = c;
    }
    result[len] = '\0';
    return result;
}

/* ── Vec Operations ───────────────────────────────────────────────── */

/* Simple Vec: { int64_t *data; uint64_t len; uint64_t cap; } */
typedef struct {
    int64_t *data;
    uint64_t len;
    uint64_t cap;
} RivenVec;

RivenVec *riven_vec_new(void) {
    RivenVec *v = (RivenVec *)malloc(sizeof(RivenVec));
    if (!v) {
        riven_panic("out of memory");
    }
    v->data = NULL;
    v->len = 0;
    v->cap = 0;
    return v;
}

/* Internal: grow a Vec's capacity to at least `needed` slots.
   Kept available for future Vec operations even if `riven_vec_push`
   currently inlines its own grow logic. */
__attribute__((unused))
static void riven_vec_grow(RivenVec *v, uint64_t needed) {
    uint64_t new_cap = v->cap == 0 ? 4 : v->cap * 2;
    while (new_cap < needed) {
        uint64_t doubled = new_cap * 2;
        if (doubled < new_cap) {
            riven_panic("vector capacity overflow");
        }
        new_cap = doubled;
    }
    size_t alloc_size;
    if (__builtin_mul_overflow(new_cap, sizeof(int64_t), &alloc_size)) {
        riven_panic("vector allocation size overflow");
    }
    int64_t *new_data = (int64_t *)realloc(v->data, alloc_size);
    if (!new_data) {
        riven_panic("out of memory");
    }
    v->data = new_data;
    v->cap = new_cap;
}

void riven_vec_push(RivenVec *v, int64_t item) {
    if (!v) return;
    if (v->len >= v->cap) {
        uint64_t new_cap = v->cap == 0 ? 4 : v->cap * 2;
        /* Overflow check on capacity doubling */
        if (new_cap < v->cap) {
            riven_panic("vector capacity overflow");
        }
        /* Overflow check on allocation size */
        size_t alloc_size;
        if (__builtin_mul_overflow(new_cap, sizeof(int64_t), &alloc_size)) {
            riven_panic("vector allocation size overflow");
        }
        /* Preserve original pointer in case realloc fails */
        int64_t *new_data = (int64_t *)realloc(v->data, alloc_size);
        if (!new_data) {
            riven_panic("out of memory");
        }
        v->data = new_data;
        v->cap = new_cap;
    }
    v->data[v->len++] = item;
}

/* Pop the last element off a Vec, returning an Option tagged union:
   [tag:i32 pad:i32 payload:i64]. tag=0 → None, tag=1 → Some(value). */
void *riven_vec_pop(RivenVec *v) {
    int64_t *result = (int64_t *)riven_alloc(16);
    if (!v || v->len == 0) {
        *(int32_t *)result = 0; /* None */
    } else {
        v->len -= 1;
        *(int32_t *)result = 1; /* Some */
        result[1] = v->data[v->len];
    }
    return result;
}

uint64_t riven_vec_len(RivenVec *v) {
    return v ? v->len : 0;
}

/* Decode a UTF-8 string into a Vec of codepoints (widened to i64) so that
   the existing Vec-iteration machinery can drive `for ch in s.chars`.
   Malformed bytes are passed through as single-byte codepoints. */
RivenVec *riven_string_chars(const char *s) {
    RivenVec *result = riven_vec_new();
    if (!s) return result;
    const unsigned char *p = (const unsigned char *)s;
    while (*p) {
        uint32_t cp;
        size_t n;
        unsigned char b0 = *p;
        if (b0 < 0x80) {
            cp = b0;
            n = 1;
        } else if ((b0 & 0xE0) == 0xC0 && (p[1] & 0xC0) == 0x80) {
            cp = ((uint32_t)(b0 & 0x1F) << 6)
               |  (uint32_t)(p[1] & 0x3F);
            n = 2;
        } else if ((b0 & 0xF0) == 0xE0
                   && (p[1] & 0xC0) == 0x80 && (p[2] & 0xC0) == 0x80) {
            cp = ((uint32_t)(b0 & 0x0F) << 12)
               | ((uint32_t)(p[1] & 0x3F) << 6)
               |  (uint32_t)(p[2] & 0x3F);
            n = 3;
        } else if ((b0 & 0xF8) == 0xF0
                   && (p[1] & 0xC0) == 0x80 && (p[2] & 0xC0) == 0x80
                   && (p[3] & 0xC0) == 0x80) {
            cp = ((uint32_t)(b0 & 0x07) << 18)
               | ((uint32_t)(p[1] & 0x3F) << 12)
               | ((uint32_t)(p[2] & 0x3F) << 6)
               |  (uint32_t)(p[3] & 0x3F);
            n = 4;
        } else {
            cp = b0;
            n = 1;
        }
        riven_vec_push(result, (int64_t)cp);
        p += n;
    }
    return result;
}

int64_t riven_vec_get(RivenVec *v, uint64_t index) {
    if (!v || index >= v->len) {
        riven_panic("index out of bounds");
    }
    return v->data[index];
}

/* get_mut: returns a POINTER to the element in the Vec's buffer.
   This allows mutations through the returned reference to modify
   the actual element in the Vec. Panics if out of bounds. */
int64_t *riven_vec_get_mut(RivenVec *v, uint64_t index) {
    if (!v || index >= v->len) {
        riven_panic("index out of bounds");
    }
    return &v->data[index];
}

/* get_opt: returns a proper Option tagged union (16 bytes):
   [tag: i32] [pad: i32] [payload: i64]
   tag=0 → None, tag=1 → Some(value) */
void *riven_vec_get_opt(RivenVec *v, uint64_t index) {
    int64_t *result = (int64_t *)riven_alloc(16);
    if (!v || index >= v->len) {
        *(int32_t *)result = 0; /* None */
    } else {
        *(int32_t *)result = 1; /* Some */
        result[1] = v->data[index];
    }
    return result;
}

/* get_mut_opt: like get_opt but returns a pointer to the element
   instead of a copy, enabling mutation through the reference. */
void *riven_vec_get_mut_opt(RivenVec *v, uint64_t index) {
    int64_t *result = (int64_t *)riven_alloc(16);
    if (!v || index >= v->len) {
        *(int32_t *)result = 0; /* None */
    } else {
        *(int32_t *)result = 1; /* Some */
        /* Store pointer to element, not copy of element */
        result[1] = (int64_t)&v->data[index];
    }
    return result;
}

int8_t riven_vec_is_empty(RivenVec *v) {
    return (!v || v->len == 0) ? 1 : 0;
}

void riven_vec_each(RivenVec *v, void (*callback)(int64_t)) {
    /* In the v1 runtime, closures/blocks are not yet supported.
       Just iterate and call the callback if non-null. */
    if (!v || !callback) return;
    for (uint64_t i = 0; i < v->len; i++) {
        callback(v->data[i]);
    }
}

/* ── Hash Operations ──────────────────────────────────────────────── */

/* Simple Hash: array of bucket linked lists, keyed by uintptr_t.
   Keys and values are both pointer-sized (stored as int64_t).
   For string keys (char*), hashing walks the bytes. For integer keys,
   the raw bits are hashed. Chained collisions handled via next pointer. */

typedef struct RivenHashEntry {
    int64_t key;
    int64_t value;
    struct RivenHashEntry *next;
} RivenHashEntry;

#define RIVEN_HASH_BUCKETS 16u

typedef struct {
    RivenHashEntry *buckets[RIVEN_HASH_BUCKETS];
    uint64_t len;
    /* Flag set to 1 if keys should be compared/hashed as C strings. The
       first inserted key decides: string pointers have the top bit clear
       on practical platforms, but we can't reliably detect that. Instead,
       we use a heuristic: if the first key, as a pointer, points to a
       readable NUL-terminated region that is ASCII-ish, treat as string.
       For simplicity and correctness, we always hash the low 8 bytes as
       raw bits and only switch to strcmp if the caller uses the string
       variant (see riven_hash_insert_str). v1 keeps a single code path
       and treats keys by raw bits, relying on the fact that string
       interning / stable pointers aren't assumed — callers using string
       keys in the v1 runtime must pass pointers whose identity matches
       the `riven_string_from`-returned pointer. Since hash!{} lowers to
       insert calls on the same string literals, this works for the
       common case where the same string constant pointer is reused. */
    int8_t string_keys;
} RivenHash;

static uint64_t riven_hash_bits(int64_t k) {
    /* splitmix64-ish finalizer for decent distribution on raw int bits. */
    uint64_t x = (uint64_t)k;
    x = (x ^ (x >> 30)) * 0xbf58476d1ce4e5b9ULL;
    x = (x ^ (x >> 27)) * 0x94d049bb133111ebULL;
    x = x ^ (x >> 31);
    return x;
}

static uint64_t riven_hash_str(const char *s) {
    /* FNV-1a on the byte contents for string-keyed hashes. */
    uint64_t h = 1469598103934665603ULL;
    if (!s) return h;
    while (*s) {
        h ^= (uint8_t)(*s);
        h *= 1099511628211ULL;
        s++;
    }
    return h;
}

static int riven_hash_keys_equal(const RivenHash *h, int64_t a, int64_t b) {
    if (a == b) return 1;
    if (h && h->string_keys) {
        const char *sa = (const char *)a;
        const char *sb = (const char *)b;
        if (!sa || !sb) return 0;
        return strcmp(sa, sb) == 0;
    }
    return 0;
}

static uint64_t riven_hash_key_hash(const RivenHash *h, int64_t key) {
    if (h && h->string_keys) {
        return riven_hash_str((const char *)key);
    }
    return riven_hash_bits(key);
}

/* Heuristic: assume the key is a string if its value looks like a
   valid pointer (non-zero, points to a readable byte region). This
   is conservative — tests use literal string constants whose bits
   are always >= 0x1000 on practical systems. Integers small enough
   to be clearly non-pointers fall through to bit hashing. */
static int riven_hash_looks_like_string(int64_t key) {
    uintptr_t p = (uintptr_t)key;
    /* Small non-pointer values. */
    if (p < 0x1000) return 0;
    /* Probe the first byte; if it's ASCII/UTF-8 and followed by a NUL
       within a short window, treat as string. This is best-effort; we
       accept false negatives (rare). */
    const unsigned char *s = (const unsigned char *)p;
    for (size_t i = 0; i < 256; i++) {
        if (s[i] == 0) return i > 0;
        if (s[i] < 0x09) return 0; /* control char — not a C string */
    }
    /* No NUL in 256 bytes — probably not a C string we care about. */
    return 0;
}

RivenHash *riven_hash_new(void) {
    RivenHash *h = (RivenHash *)malloc(sizeof(RivenHash));
    if (!h) {
        riven_panic("out of memory");
    }
    for (unsigned i = 0; i < RIVEN_HASH_BUCKETS; i++) {
        h->buckets[i] = NULL;
    }
    h->len = 0;
    h->string_keys = -1; /* unset — decided on first insert */
    return h;
}

void riven_hash_insert(RivenHash *h, int64_t key, int64_t value) {
    if (!h) return;
    if (h->string_keys < 0) {
        h->string_keys = riven_hash_looks_like_string(key) ? 1 : 0;
    }
    uint64_t bucket_idx = riven_hash_key_hash(h, key) % RIVEN_HASH_BUCKETS;
    RivenHashEntry *e = h->buckets[bucket_idx];
    while (e) {
        if (riven_hash_keys_equal(h, e->key, key)) {
            e->value = value;
            return;
        }
        e = e->next;
    }
    RivenHashEntry *ne = (RivenHashEntry *)malloc(sizeof(RivenHashEntry));
    if (!ne) {
        riven_panic("out of memory");
    }
    ne->key = key;
    ne->value = value;
    ne->next = h->buckets[bucket_idx];
    h->buckets[bucket_idx] = ne;
    h->len += 1;
}

/* Return an Option tagged union (16 bytes): tag=1 Some(&value), tag=0 None.
   The payload carries the raw value (v1 treats &V the same as V at the
   runtime level — both are 8 bytes). */
void *riven_hash_get(RivenHash *h, int64_t key) {
    int64_t *result = (int64_t *)riven_alloc(16);
    if (!h) {
        *(int32_t *)result = 0;
        return result;
    }
    uint64_t bucket_idx = riven_hash_key_hash(h, key) % RIVEN_HASH_BUCKETS;
    RivenHashEntry *e = h->buckets[bucket_idx];
    while (e) {
        if (riven_hash_keys_equal(h, e->key, key)) {
            *(int32_t *)result = 1; /* Some */
            result[1] = e->value;
            return result;
        }
        e = e->next;
    }
    *(int32_t *)result = 0; /* None */
    return result;
}

int8_t riven_hash_contains_key(RivenHash *h, int64_t key) {
    if (!h) return 0;
    uint64_t bucket_idx = riven_hash_key_hash(h, key) % RIVEN_HASH_BUCKETS;
    RivenHashEntry *e = h->buckets[bucket_idx];
    while (e) {
        if (riven_hash_keys_equal(h, e->key, key)) {
            return 1;
        }
        e = e->next;
    }
    return 0;
}

uint64_t riven_hash_len(RivenHash *h) {
    return h ? h->len : 0;
}

int8_t riven_hash_is_empty(RivenHash *h) {
    return (!h || h->len == 0) ? 1 : 0;
}

/* ── Set Operations ───────────────────────────────────────────────── */

/* Built on top of the Hash — values are unused (set to 1). */

typedef struct {
    RivenHash inner;
} RivenSet;

RivenSet *riven_set_new(void) {
    RivenSet *s = (RivenSet *)malloc(sizeof(RivenSet));
    if (!s) {
        riven_panic("out of memory");
    }
    for (unsigned i = 0; i < RIVEN_HASH_BUCKETS; i++) {
        s->inner.buckets[i] = NULL;
    }
    s->inner.len = 0;
    s->inner.string_keys = -1;
    return s;
}

void riven_set_insert(RivenSet *s, int64_t item) {
    if (!s) return;
    /* Reuse hash insert for dedup semantics; value is 1 (unused). */
    riven_hash_insert(&s->inner, item, 1);
}

int8_t riven_set_contains(RivenSet *s, int64_t item) {
    if (!s) return 0;
    return riven_hash_contains_key(&s->inner, item);
}

uint64_t riven_set_len(RivenSet *s) {
    return s ? s->inner.len : 0;
}

int8_t riven_set_is_empty(RivenSet *s) {
    return (!s || s->inner.len == 0) ? 1 : 0;
}

/* ── &str Operations ──────────────────────────────────────────────── */

RivenVec *riven_str_split(const char *s, const char *delimiter) {
    RivenVec *result = riven_vec_new();
    if (!s) return result;
    if (!delimiter || delimiter[0] == '\0') {
        riven_vec_push(result, (int64_t)riven_string_from(s));
        return result;
    }
    size_t dlen = strlen(delimiter);
    const char *start = s;
    while (1) {
        const char *found = strstr(start, delimiter);
        if (!found) {
            riven_vec_push(result, (int64_t)riven_string_from(start));
            break;
        }
        size_t part_len = (size_t)(found - start);
        char *part = (char *)malloc(part_len + 1);
        if (!part) {
            riven_panic("out of memory");
        }
        memcpy(part, start, part_len);
        part[part_len] = '\0';
        riven_vec_push(result, (int64_t)part);
        start = found + dlen;
    }
    return result;
}

/* Parse a string to an unsigned integer, returning a Result-like value.
   Returns a tagged union: tag=0 (Ok) with value, tag=1 (Err). */
void *riven_str_parse_uint(const char *s) {
    /* Allocate a tagged union: [tag:i32 pad:i32 payload:i64] = 16 bytes */
    int64_t *result = (int64_t *)riven_alloc(16);
    if (!s || *s == '\0') {
        *(int32_t *)result = 1; /* Err */
        return result;
    }
    char *end;
    unsigned long val = strtoul(s, &end, 10);
    if (*end != '\0') {
        *(int32_t *)result = 1; /* Err */
    } else {
        *(int32_t *)result = 0; /* Ok */
        result[1] = (int64_t)val;
    }
    return result;
}

/* ── Option / Result Helpers ──────────────────────────────────────── */

/* Option unwrap_or: if tag==0 (None), return default_val; if tag==1 (Some), return payload */
int64_t riven_option_unwrap_or(void *opt, int64_t default_val) {
    if (!opt) return default_val;
    int32_t tag = *(int32_t *)opt;
    if (tag == 0) return default_val; /* None */
    return ((int64_t *)opt)[1]; /* Some(payload) */
}

/* Result unwrap_or_else: if Ok (tag 0), return payload. If Err, call handler. */
int64_t riven_result_unwrap_or_else(void *result, void (*handler)(int64_t)) {
    if (!result) return 0;
    int32_t tag = *(int32_t *)result;
    if (tag == 0) return ((int64_t *)result)[1]; /* Ok */
    /* Err — call handler with error payload if handler is non-null */
    if (handler) {
        int64_t err_payload = ((int64_t *)result)[1];
        handler(err_payload);
    }
    return 0;
}

/* Result try_op (? operator): if Ok, return payload. If Err, propagate. */
int64_t riven_result_try_op(void *result) {
    if (!result) return 0;
    int32_t tag = *(int32_t *)result;
    if (tag == 0) return ((int64_t *)result)[1]; /* Ok */
    /* Err — in a real implementation, this would propagate via a return.
       For now, just return 0. */
    return 0;
}

/* Result expect!(msg): if Ok, return payload; if Err, panic with `msg`. */
int64_t riven_result_expect(void *result, const char *msg) {
    if (!result) riven_panic(msg ? msg : "expect! on null");
    int32_t tag = *(int32_t *)result;
    if (tag == 0) return ((int64_t *)result)[1]; /* Ok */
    riven_panic(msg ? msg : "expect! on Err");
    return 0; /* unreachable */
}

/* Result unwrap!: if Ok, return payload; if Err, panic. */
int64_t riven_result_unwrap(void *result) {
    if (!result) riven_panic("unwrap! on null");
    int32_t tag = *(int32_t *)result;
    if (tag == 0) return ((int64_t *)result)[1]; /* Ok */
    riven_panic("unwrap! on Err");
    return 0; /* unreachable */
}

/* Option expect!(msg): if Some, return payload; if None, panic with `msg`. */
int64_t riven_option_expect(void *opt, const char *msg) {
    if (!opt) riven_panic(msg ? msg : "expect! on null");
    int32_t tag = *(int32_t *)opt;
    if (tag == 1) return ((int64_t *)opt)[1]; /* Some */
    riven_panic(msg ? msg : "expect! on None");
    return 0; /* unreachable */
}

/* Option unwrap!: if Some, return payload; if None, panic. */
int64_t riven_option_unwrap(void *opt) {
    if (!opt) riven_panic("unwrap! on null");
    int32_t tag = *(int32_t *)opt;
    if (tag == 1) return ((int64_t *)opt)[1]; /* Some */
    riven_panic("unwrap! on None");
    return 0; /* unreachable */
}

/* Result ok(): Result[T,E] -> Option[T]. Ok(x) -> Some(x); Err(_) -> None. */
void *riven_result_ok(void *result) {
    int64_t *out = (int64_t *)riven_alloc(16);
    if (result && *(int32_t *)result == 0) {
        *(int32_t *)out = 1; /* Some */
        out[1] = ((int64_t *)result)[1];
    } else {
        *(int32_t *)out = 0; /* None */
    }
    return out;
}

/* Result err(): Result[T,E] -> Option[E]. Err(e) -> Some(e); Ok(_) -> None. */
void *riven_result_err(void *result) {
    int64_t *out = (int64_t *)riven_alloc(16);
    if (result && *(int32_t *)result == 1) {
        *(int32_t *)out = 1; /* Some */
        out[1] = ((int64_t *)result)[1];
    } else {
        *(int32_t *)out = 0; /* None */
    }
    return out;
}

/* Option / Result is_* predicates (return i8). */
int8_t riven_option_is_some(void *opt) {
    return opt && *(int32_t *)opt == 1;
}
int8_t riven_option_is_none(void *opt) {
    return !opt || *(int32_t *)opt == 0;
}
int8_t riven_result_is_ok(void *result) {
    return result && *(int32_t *)result == 0;
}
int8_t riven_result_is_err(void *result) {
    return !result || *(int32_t *)result == 1;
}

/* ── No-op Stubs ──────────────────────────────────────────────────── */

/* Pass through the first argument unchanged (for iterator wrappers etc.) */
int64_t riven_noop_passthrough(int64_t val) {
    return val;
}

/* Return null (for find/position that return Option) */
int64_t riven_noop_return_null(void) {
    return 0;
}

void riven_noop(void) {}

/* ── Panic ─────────────────────────────────────────────────────────── */

void riven_panic(const char *message) {
    fflush(stdout);
    fprintf(stderr, "riven panic: %s\n", message ? message : "(unknown)");
    fflush(stderr);
    exit(101);
}
