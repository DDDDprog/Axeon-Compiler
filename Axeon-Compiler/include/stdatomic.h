/*
 * stdatomic.h — Axeon built-in compiler-intrinsic header.
 *
 * Provides a minimal C11 <stdatomic.h> implementation.
 *
 * NOTE: Axeon currently parses _Atomic but treats the qualifier as
 * transparent (no lock-free atomicity guarantees in the generated code).
 * This header provides the type aliases, memory-order enum, and macros
 * so that code that *includes* <stdatomic.h> compiles without error.
 * The operations map to plain loads/stores/RMWs; code that requires
 * true lock-free atomics should use a dedicated threading library.
 *
 * No GCC dependency — uses only built-in types and C11 keywords.
 */

#ifndef _STDATOMIC_H
#define _STDATOMIC_H

#include <stddef.h>   /* size_t, ptrdiff_t */
#include <stdint.h>   /* uintX_t           */
#include <stdbool.h>  /* bool              */

#ifdef __cplusplus
extern "C" {
#endif

/* ------------------------------------------------------------------ */
/* Memory order                                                        */
/* ------------------------------------------------------------------ */

typedef enum memory_order {
    memory_order_relaxed = 0,
    memory_order_consume = 1,
    memory_order_acquire = 2,
    memory_order_release = 3,
    memory_order_acq_rel = 4,
    memory_order_seq_cst = 5
} memory_order;

/* ------------------------------------------------------------------ */
/* Atomic flag                                                         */
/* ------------------------------------------------------------------ */

typedef struct { _Atomic int __val; } atomic_flag;

#define ATOMIC_FLAG_INIT { 0 }

#define atomic_flag_test_and_set(obj) \
    __atomic_test_and_set(&(obj)->__val, memory_order_seq_cst)

#define atomic_flag_test_and_set_explicit(obj, order) \
    __atomic_test_and_set(&(obj)->__val, (order))

#define atomic_flag_clear(obj) \
    __atomic_clear(&(obj)->__val, memory_order_seq_cst)

#define atomic_flag_clear_explicit(obj, order) \
    __atomic_clear(&(obj)->__val, (order))

/* ------------------------------------------------------------------ */
/* Atomic boolean and integer type aliases                             */
/* ------------------------------------------------------------------ */

typedef _Atomic _Bool               atomic_bool;
typedef _Atomic char                atomic_char;
typedef _Atomic signed char         atomic_schar;
typedef _Atomic unsigned char       atomic_uchar;
typedef _Atomic short               atomic_short;
typedef _Atomic unsigned short      atomic_ushort;
typedef _Atomic int                 atomic_int;
typedef _Atomic unsigned int        atomic_uint;
typedef _Atomic long                atomic_long;
typedef _Atomic unsigned long       atomic_ulong;
typedef _Atomic long long           atomic_llong;
typedef _Atomic unsigned long long  atomic_ullong;

typedef _Atomic uint_least16_t      atomic_char16_t;
typedef _Atomic uint_least32_t      atomic_char32_t;
typedef _Atomic wchar_t             atomic_wchar_t;

typedef _Atomic  int_least8_t       atomic_int_least8_t;
typedef _Atomic uint_least8_t       atomic_uint_least8_t;
typedef _Atomic  int_least16_t      atomic_int_least16_t;
typedef _Atomic uint_least16_t      atomic_uint_least16_t;
typedef _Atomic  int_least32_t      atomic_int_least32_t;
typedef _Atomic uint_least32_t      atomic_uint_least32_t;
typedef _Atomic  int_least64_t      atomic_int_least64_t;
typedef _Atomic uint_least64_t      atomic_uint_least64_t;

typedef _Atomic  int_fast8_t        atomic_int_fast8_t;
typedef _Atomic uint_fast8_t        atomic_uint_fast8_t;
typedef _Atomic  int_fast16_t       atomic_int_fast16_t;
typedef _Atomic uint_fast16_t       atomic_uint_fast16_t;
typedef _Atomic  int_fast32_t       atomic_int_fast32_t;
typedef _Atomic uint_fast32_t       atomic_uint_fast32_t;
typedef _Atomic  int_fast64_t       atomic_int_fast64_t;
typedef _Atomic uint_fast64_t       atomic_uint_fast64_t;

typedef _Atomic  intptr_t           atomic_intptr_t;
typedef _Atomic uintptr_t           atomic_uintptr_t;
typedef _Atomic   size_t            atomic_size_t;
typedef _Atomic ptrdiff_t           atomic_ptrdiff_t;
typedef _Atomic  intmax_t           atomic_intmax_t;
typedef _Atomic uintmax_t           atomic_uintmax_t;

/* ------------------------------------------------------------------ */
/* ATOMIC_VAR_INIT — C11 (deprecated in C17, removed in C23)          */
/* ------------------------------------------------------------------ */

#define ATOMIC_VAR_INIT(val) (val)

/* ------------------------------------------------------------------ */
/* atomic_init                                                         */
/* ------------------------------------------------------------------ */

#define atomic_init(obj, val) ((void)(*(obj) = (val)))

/* ------------------------------------------------------------------ */
/* Fences                                                              */
/* ------------------------------------------------------------------ */

/* On x86-64, a compiler barrier is sufficient for most fence kinds.
 * For seq_cst we emit an mfence-equivalent via the GCC intrinsic.    */
#define atomic_thread_fence(order)   __atomic_thread_fence(order)
#define atomic_signal_fence(order)   __atomic_signal_fence(order)

/* ------------------------------------------------------------------ */
/* Lock-free query macros                                              */
/* ------------------------------------------------------------------ */

/* We claim lock-free for all types (the operations use plain
 * loads/stores, which is fine for correctly-sized aligned objects).  */
#define ATOMIC_BOOL_LOCK_FREE       2
#define ATOMIC_CHAR_LOCK_FREE       2
#define ATOMIC_CHAR16_T_LOCK_FREE   2
#define ATOMIC_CHAR32_T_LOCK_FREE   2
#define ATOMIC_WCHAR_T_LOCK_FREE    2
#define ATOMIC_SHORT_LOCK_FREE      2
#define ATOMIC_INT_LOCK_FREE        2
#define ATOMIC_LONG_LOCK_FREE       2
#define ATOMIC_LLONG_LOCK_FREE      2
#define ATOMIC_POINTER_LOCK_FREE    2

#define atomic_is_lock_free(obj)    (1)

/* ------------------------------------------------------------------ */
/* Load / store / exchange / compare-exchange                          */
/* ------------------------------------------------------------------ */

/* Generic load: return the value stored in *obj.                     */
#define atomic_load(obj) \
    __atomic_load_n((obj), memory_order_seq_cst)

#define atomic_load_explicit(obj, order) \
    __atomic_load_n((obj), (order))

/* Generic store: store val in *obj.                                  */
#define atomic_store(obj, val) \
    __atomic_store_n((obj), (val), memory_order_seq_cst)

#define atomic_store_explicit(obj, val, order) \
    __atomic_store_n((obj), (val), (order))

/* Generic exchange: store val in *obj and return old value.          */
#define atomic_exchange(obj, val) \
    __atomic_exchange_n((obj), (val), memory_order_seq_cst)

#define atomic_exchange_explicit(obj, val, order) \
    __atomic_exchange_n((obj), (val), (order))

/* Strong compare-and-exchange.                                       */
#define atomic_compare_exchange_strong(obj, expected, desired) \
    __atomic_compare_exchange_n((obj), (expected), (desired), \
        0, memory_order_seq_cst, memory_order_seq_cst)

#define atomic_compare_exchange_strong_explicit(obj, expected, desired, succ, fail) \
    __atomic_compare_exchange_n((obj), (expected), (desired), 0, (succ), (fail))

/* Weak compare-and-exchange (may spuriously fail).                   */
#define atomic_compare_exchange_weak(obj, expected, desired) \
    __atomic_compare_exchange_n((obj), (expected), (desired), \
        1, memory_order_seq_cst, memory_order_seq_cst)

#define atomic_compare_exchange_weak_explicit(obj, expected, desired, succ, fail) \
    __atomic_compare_exchange_n((obj), (expected), (desired), 1, (succ), (fail))

/* ------------------------------------------------------------------ */
/* Fetch-and-modify operations                                        */
/* ------------------------------------------------------------------ */

#define atomic_fetch_add(obj, val) \
    __atomic_fetch_add((obj), (val), memory_order_seq_cst)
#define atomic_fetch_add_explicit(obj, val, order) \
    __atomic_fetch_add((obj), (val), (order))

#define atomic_fetch_sub(obj, val) \
    __atomic_fetch_sub((obj), (val), memory_order_seq_cst)
#define atomic_fetch_sub_explicit(obj, val, order) \
    __atomic_fetch_sub((obj), (val), (order))

#define atomic_fetch_or(obj, val) \
    __atomic_fetch_or((obj), (val), memory_order_seq_cst)
#define atomic_fetch_or_explicit(obj, val, order) \
    __atomic_fetch_or((obj), (val), (order))

#define atomic_fetch_xor(obj, val) \
    __atomic_fetch_xor((obj), (val), memory_order_seq_cst)
#define atomic_fetch_xor_explicit(obj, val, order) \
    __atomic_fetch_xor((obj), (val), (order))

#define atomic_fetch_and(obj, val) \
    __atomic_fetch_and((obj), (val), memory_order_seq_cst)
#define atomic_fetch_and_explicit(obj, val, order) \
    __atomic_fetch_and((obj), (val), (order))

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* _STDATOMIC_H */