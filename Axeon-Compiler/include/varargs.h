/*
 * varargs.h — Axeon built-in compiler-intrinsic header.
 *
 * varargs.h is the pre-ANSI predecessor of <stdarg.h>.  It is deprecated
 * and should not be used in new code, but some legacy codebases (and a few
 * system headers that defensively include it) still reference it.
 *
 * This implementation simply delegates to <stdarg.h>.
 * No GCC dependency.
 */

#ifndef _VARARGS_H
#define _VARARGS_H

#include <stdarg.h>

/*
 * Pre-ANSI varargs.h defined va_alist / va_dcl for K&R-style variadic
 * functions.  Modern compilers (including Axeon) do not support K&R
 * function definitions, so we provide only empty compatibility stubs to
 * avoid preprocessor errors in headers that test for these macros.
 */
#ifndef va_alist
# define va_alist   __va_alist
#endif

#ifndef va_dcl
# define va_dcl     int __va_alist;
#endif

#endif /* _VARARGS_H */