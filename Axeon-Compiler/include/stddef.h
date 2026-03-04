/*
 * stddef.h — Axeon built-in compiler-intrinsic header.
 *
 * Provides: size_t, ptrdiff_t, wchar_t, NULL, offsetof, max_align_t.
 * Uses only predefined compiler macros (__SIZE_TYPE__ etc.) — no GCC dependency.
 */

#ifndef _STDDEF_H
#define _STDDEF_H

/* ------------------------------------------------------------------ */
/* Fundamental types                                                   */
/* ------------------------------------------------------------------ */

#ifndef __size_t_defined
#ifndef __SIZE_TYPE__
# define __SIZE_TYPE__ unsigned long
#endif
typedef __SIZE_TYPE__ size_t;
#define __size_t_defined
#endif

#ifndef __ptrdiff_t_defined
#ifndef __PTRDIFF_TYPE__
# define __PTRDIFF_TYPE__ long
#endif
typedef __PTRDIFF_TYPE__ ptrdiff_t;
#define __ptrdiff_t_defined
#endif

#ifndef __wchar_t_defined
#ifndef __WCHAR_TYPE__
# define __WCHAR_TYPE__ int
#endif
#ifndef __cplusplus
typedef __WCHAR_TYPE__ wchar_t;
#endif
#define __wchar_t_defined
#endif

/* ------------------------------------------------------------------ */
/* NULL                                                                */
/* ------------------------------------------------------------------ */

#ifndef NULL
# ifdef __cplusplus
#  if __cplusplus >= 201103L
#   define NULL nullptr
#  else
#   define NULL 0L
#  endif
# else
#  define NULL ((void *)0)
# endif
#endif

/* ------------------------------------------------------------------ */
/* offsetof                                                            */
/* ------------------------------------------------------------------ */

#ifdef __builtin_offsetof
# define offsetof(type, member) __builtin_offsetof(type, member)
#else
# define offsetof(type, member) ((size_t)((char *)&((type *)0)->member - (char *)0))
#endif

/* ------------------------------------------------------------------ */
/* max_align_t (C11 / C++11)                                          */
/* ------------------------------------------------------------------ */

#if (defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L) || \
    (defined(__cplusplus)      && __cplusplus      >= 201103L)
# ifndef __max_align_t_defined
typedef struct {
    long long          __max_align_ll __attribute__((__aligned__(__alignof__(long long))));
    long double        __max_align_ld __attribute__((__aligned__(__alignof__(long double))));
} max_align_t;
#  define __max_align_t_defined
# endif
#endif

/* ------------------------------------------------------------------ */
/* unreachable() — C23                                                */
/* ------------------------------------------------------------------ */

#if defined(__STDC_VERSION__) && __STDC_VERSION__ >= 202311L
# ifndef unreachable
#  define unreachable() __builtin_unreachable()
# endif
#endif

#endif /* _STDDEF_H */