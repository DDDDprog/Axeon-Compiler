/*
 * stdarg.h — Axeon built-in compiler-intrinsic header.
 *
 * Provides: va_list, va_start, va_arg, va_end, va_copy.
 * Uses __builtin_va_* which Axeon handles natively — no GCC dependency.
 */

#ifndef _STDARG_H
#define _STDARG_H

#ifdef __cplusplus
extern "C" {
#endif

/* ------------------------------------------------------------------ */
/* va_list type                                                         */
/* ------------------------------------------------------------------ */

/* __builtin_va_list, __gnuc_va_list, and va_list are pre-injected by
 * the Axeon compiler as built-in typedef names (void* in its internal
 * type model).  We still emit the typedef here so that code which
 * compiles this header without the Axeon compiler (e.g. static
 * analysis tools) gets a sane definition.                             */

#ifndef __GNUC_VA_LIST
# define __GNUC_VA_LIST
  typedef __builtin_va_list __gnuc_va_list;
#endif

#ifndef _VA_LIST_DEFINED
# define _VA_LIST_DEFINED
  typedef __gnuc_va_list va_list;
#endif

/* Some glibc / musl headers use __isoc_va_list */
#ifndef __isoc_va_list_defined
# define __isoc_va_list_defined
  typedef __gnuc_va_list __isoc_va_list;
#endif

/* ------------------------------------------------------------------ */
/* Core macros                                                          */
/* ------------------------------------------------------------------ */

#define va_start(ap, last)   __builtin_va_start(ap, last)
#define va_end(ap)           __builtin_va_end(ap)
#define va_arg(ap, type)     __builtin_va_arg(ap, type)
#define va_copy(dest, src)   __builtin_va_copy(dest, src)

/* C99 §7.15.1.1: va_list is an array or struct type on some ABIs.
 * The __va_copy alias predates C99 and is still used in older code.  */
#ifndef __va_copy
# define __va_copy(dest, src) va_copy(dest, src)
#endif

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* _STDARG_H */