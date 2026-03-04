/*
 * limits.h — Axeon built-in compiler-intrinsic header.
 *
 * Provides all C89/C99/C11 integer limit macros.
 * Uses only the __*_MAX__ / __CHAR_BIT__ predefined macros that
 * Axeon (and GCC) inject automatically — no GCC dependency.
 */

#ifndef _LIMITS_H
#define _LIMITS_H

/* ------------------------------------------------------------------ */
/* Char                                                                */
/* ------------------------------------------------------------------ */

#define CHAR_BIT    __CHAR_BIT__

#ifdef __CHAR_UNSIGNED__
# define CHAR_MIN   0
# define CHAR_MAX   __SCHAR_MAX__
#else
# define CHAR_MIN   (-__SCHAR_MAX__ - 1)
# define CHAR_MAX   __SCHAR_MAX__
#endif

/* ------------------------------------------------------------------ */
/* signed char                                                         */
/* ------------------------------------------------------------------ */

#define SCHAR_MIN   (-__SCHAR_MAX__ - 1)
#define SCHAR_MAX   __SCHAR_MAX__

/* ------------------------------------------------------------------ */
/* unsigned char                                                       */
/* ------------------------------------------------------------------ */

#define UCHAR_MAX   (__SCHAR_MAX__ * 2 + 1)

/* ------------------------------------------------------------------ */
/* short                                                               */
/* ------------------------------------------------------------------ */

#define SHRT_MIN    (-__SHRT_MAX__ - 1)
#define SHRT_MAX    __SHRT_MAX__
#define USHRT_MAX   (__SHRT_MAX__ * 2 + 1)

/* ------------------------------------------------------------------ */
/* int                                                                 */
/* ------------------------------------------------------------------ */

#define INT_MIN     (-__INT_MAX__ - 1)
#define INT_MAX     __INT_MAX__
#define UINT_MAX    (__INT_MAX__ * 2U + 1U)

/* ------------------------------------------------------------------ */
/* long                                                                */
/* ------------------------------------------------------------------ */

#define LONG_MIN    (-__LONG_MAX__ - 1L)
#define LONG_MAX    __LONG_MAX__
#define ULONG_MAX   (__LONG_MAX__ * 2UL + 1UL)

/* ------------------------------------------------------------------ */
/* long long (C99)                                                     */
/* ------------------------------------------------------------------ */

#define LLONG_MIN   (-__LONG_LONG_MAX__ - 1LL)
#define LLONG_MAX   __LONG_LONG_MAX__
#define ULLONG_MAX  (__LONG_LONG_MAX__ * 2ULL + 1ULL)

/* ------------------------------------------------------------------ */
/* POSIX / XSI extensions                                              */
/* ------------------------------------------------------------------ */

/* Maximum bytes in a multibyte character */
#ifndef MB_LEN_MAX
# define MB_LEN_MAX 16
#endif

/* ------------------------------------------------------------------ */
/* PATH / NAME limits (POSIX) — conservative portable values          */
/* ------------------------------------------------------------------ */

#ifndef PATH_MAX
# define PATH_MAX   4096
#endif

#ifndef NAME_MAX
# define NAME_MAX   255
#endif

#ifndef PIPE_BUF
# define PIPE_BUF   4096
#endif

/* ------------------------------------------------------------------ */
/* NL_ARGMAX / NL_MSGMAX (POSIX optional)                             */
/* ------------------------------------------------------------------ */

#ifndef NL_ARGMAX
# define NL_ARGMAX  9
#endif

#endif /* _LIMITS_H */