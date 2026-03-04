/*
 * iso646.h — Axeon built-in compiler-intrinsic header.
 *
 * Provides alternative spellings for C logical/bitwise operators (C95 §4.2).
 * In C++, these are keywords; in C, they are macros.
 * No GCC dependency.
 */

#ifndef _ISO646_H
#define _ISO646_H

#ifndef __cplusplus

#define and    &&
#define and_eq &=
#define bitand &
#define bitor  |
#define compl  ~
#define not    !
#define not_eq !=
#define or     ||
#define or_eq  |=
#define xor    ^
#define xor_eq ^=

#endif /* !__cplusplus */

#endif /* _ISO646_H */