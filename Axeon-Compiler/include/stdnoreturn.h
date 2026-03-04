/*
 * stdnoreturn.h — Axeon built-in compiler-intrinsic header.
 *
 * Provides: noreturn macro (C11 §7.23).
 * _Noreturn is a C11 keyword handled natively by Axeon.
 * No GCC dependency.
 */

#ifndef _STDNORETURN_H
#define _STDNORETURN_H

#ifndef __cplusplus

# ifndef noreturn
#  define noreturn _Noreturn
# endif

#endif /* !__cplusplus */

#define __noreturn_is_defined 1

#endif /* _STDNORETURN_H */