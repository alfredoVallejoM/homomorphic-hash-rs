# ADR 0001 — Migración mediante workspace

**Estado:** aceptado.

## Contexto

El repositorio contiene una biblioteca legada extensa cuya API, dependencias y
objetivos no cumplen el núcleo `no_std` de Microfield.

## Decisión

Conservar el paquete raíz y añadir `crates/microfield` como segundo miembro.
Microfield continúa siendo un único paquete para sus fases 0–2.

## Consecuencias

- La migración puede comprobar compatibilidad sin romper el legado.
- Los fallos de ejemplos/benchmarks legados no bloquean el paquete nuevo.
- Una futura retirada del paquete legado requerirá otro ADR.
