/*
 * stdalign.h — Axeon built-in compiler-intrinsic header.
 *
 * Provides: alignas, alignof, __alignas_is_defined, __alignof_is_defined.
 * _Alignas and _Alignof are C11 keywords handled natively by Axeon.
 * No GCC dependency.
 */

#ifndef _STDALIGN_H
#define _STDALIGN_H

#ifndef __cplusplus

# ifndef alignas
#  define alignas _Alignas
# endif

# ifndef alignof
#  define alignof _Alignof
# endif

#endif /* !__cplusplus */

#define __alignas_is_defined 1
#define __alignof_is_defined 1

#endif /* _STDALIGN_H */