/*
 * stdbool.h — Axeon built-in compiler-intrinsic header.
 *
 * Provides: bool, true, false, __bool_true_false_are_defined.
 * No GCC dependency — uses only _Bool which is a C99 keyword.
 */

#ifndef _STDBOOL_H
#define _STDBOOL_H

#ifdef __cplusplus

/* In C++, bool/true/false are keywords — nothing to define */

#else /* C */

/* _Bool is a C99 built-in type keyword; the compiler handles it natively */
# ifndef bool
#  define bool  _Bool
# endif

# ifndef true
#  define true  1
# endif

# ifndef false
#  define false 0
# endif

#endif /* __cplusplus */

/* Signal to code that checks for this macro that our definitions are real */
#define __bool_true_false_are_defined 1

#endif /* _STDBOOL_H */