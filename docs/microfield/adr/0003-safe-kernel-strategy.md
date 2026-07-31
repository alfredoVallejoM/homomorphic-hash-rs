# ADR 0003 — Estrategias de kernel seguras

**Estado:** aceptado.

## Contexto

Los punteros raw en el ABI común obligaban a introducir `unsafe` durante la
Fase 1 y mezclaban selección con precondiciones de memoria.

## Decisión

`KernelSet` almacenará funciones seguras sobre slices. `Engine` valida
longitudes una vez. Los backends ISA futuros tendrán wrappers seguros que
encapsulen sus intrinsics y precondiciones.

## Consecuencias

- El portable mantiene `forbid(unsafe_code)`.
- El borrow checker expresa aliasing y validez.
- El coste del wrapper se somete a benchmark y desensamblado.
