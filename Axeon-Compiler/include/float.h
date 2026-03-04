/*
 * float.h — Axeon built-in compiler-intrinsic header.
 *
 * Provides all C99/C11 floating-point characteristic macros.
 * Uses only the __FLT_* / __DBL_* / __LDBL_* predefined macros that
 * Axeon (and GCC) inject automatically — no GCC dependency.
 */

#ifndef _FLOAT_H
#define _FLOAT_H

/* ------------------------------------------------------------------ */
/* Radix of the exponent representation                                */
/* ------------------------------------------------------------------ */

#define FLT_RADIX   __FLT_RADIX__

/* ------------------------------------------------------------------ */
/* float (FLT)                                                         */
/* ------------------------------------------------------------------ */

#define FLT_MANT_DIG        __FLT_MANT_DIG__
#define FLT_DIG             __FLT_DIG__
#define FLT_MIN_EXP         __FLT_MIN_EXP__
#define FLT_MIN_10_EXP      __FLT_MIN_10_EXP__
#define FLT_MAX_EXP         __FLT_MAX_EXP__
#define FLT_MAX_10_EXP      __FLT_MAX_10_EXP__
#define FLT_MAX             __FLT_MAX__
#define FLT_MIN             __FLT_MIN__
#define FLT_EPSILON         __FLT_EPSILON__
#define FLT_DECIMAL_DIG     __FLT_DECIMAL_DIG__
#define FLT_HAS_SUBNORM     1

/* C99: smallest positive subnormal */
#ifndef FLT_TRUE_MIN
# define FLT_TRUE_MIN       __FLT_DENORM_MIN__
#endif

/* ------------------------------------------------------------------ */
/* double (DBL)                                                        */
/* ------------------------------------------------------------------ */

#define DBL_MANT_DIG        __DBL_MANT_DIG__
#define DBL_DIG             __DBL_DIG__
#define DBL_MIN_EXP         __DBL_MIN_EXP__
#define DBL_MIN_10_EXP      __DBL_MIN_10_EXP__
#define DBL_MAX_EXP         __DBL_MAX_EXP__
#define DBL_MAX_10_EXP      __DBL_MAX_10_EXP__
#define DBL_MAX             __DBL_MAX__
#define DBL_MIN             __DBL_MIN__
#define DBL_EPSILON         __DBL_EPSILON__
#define DBL_DECIMAL_DIG     __DBL_DECIMAL_DIG__
#define DBL_HAS_SUBNORM     1

#ifndef DBL_TRUE_MIN
# define DBL_TRUE_MIN       __DBL_DENORM_MIN__
#endif

/* ------------------------------------------------------------------ */
/* long double (LDBL)                                                  */
/* ------------------------------------------------------------------ */

#define LDBL_MANT_DIG       __LDBL_MANT_DIG__
#define LDBL_DIG            __LDBL_DIG__
#define LDBL_MIN_EXP        __LDBL_MIN_EXP__
#define LDBL_MIN_10_EXP     __LDBL_MIN_10_EXP__
#define LDBL_MAX_EXP        __LDBL_MAX_EXP__
#define LDBL_MAX_10_EXP     __LDBL_MAX_10_EXP__
#define LDBL_MAX            __LDBL_MAX__
#define LDBL_MIN            __LDBL_MIN__
#define LDBL_EPSILON        __LDBL_EPSILON__
#define LDBL_DECIMAL_DIG    __LDBL_DECIMAL_DIG__
#define LDBL_HAS_SUBNORM    1

#ifndef LDBL_TRUE_MIN
# define LDBL_TRUE_MIN      __LDBL_DENORM_MIN__
#endif

/* ------------------------------------------------------------------ */
/* Decimal precision (C99 §5.2.4.2.2)                                 */
/* DECIMAL_DIG: rounding-trip decimal digits for the widest type      */
/* ------------------------------------------------------------------ */

#define DECIMAL_DIG         __DECIMAL_DIG__

/* ------------------------------------------------------------------ */
/* Evaluation method (C99 Annex F)                                    */
/* 0 = evaluate to the range/precision of the type                    */
/* ------------------------------------------------------------------ */

#ifndef FLT_EVAL_METHOD
# ifdef __FLT_EVAL_METHOD__
#  define FLT_EVAL_METHOD   __FLT_EVAL_METHOD__
# else
#  define FLT_EVAL_METHOD   0
# endif
#endif

/* ------------------------------------------------------------------ */
/* Rounding mode (C99 §5.2.4.2.2)                                     */
/* 1 = round to nearest (default for IEEE 754)                        */
/* ------------------------------------------------------------------ */

#ifndef FLT_ROUNDS
# define FLT_ROUNDS         1
#endif

#endif /* _FLOAT_H */