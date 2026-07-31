# Plan de Fase 1

## H0 — Scaffold

- Workspace, higiene de Git y paquete `microfield`.
- Traits segregados, IDs, errores y `F2`.
- Manifiestos y documentación.
- `std`, `no_std`, tests, formato y Clippy.

Salida: arquitectura compilable sin fingir campos grandes implementados.

## H1 — Fase 0 mínima

**Estado: implementado y validado localmente (30 de julio de 2026).**

- Modelo TOML v1 limitado a campos binarios polinómicos.
- Normalización e identidad deterministas.
- Rabin, certificados y planes.
- Puertos de artefactos/oráculo y adaptadores de filesystem/Sage.
- Emisión transaccional y regeneración con diff vacío.

Salida alcanzada: los tres manifiestos producen certificados y artefactos
verificables. Los tests golden congelan sus `FieldId`; el adaptador Sage y su
script se han ejecutado con SageMath 10.7 y los tres juegos v2 son
reproducibles byte a byte.

## H1.5 — Estabilización

- **Implementado localmente:** esquema tipado v2 de vectores externos.
- **Implementado localmente:** semántica de `ArtifactId` y
  `ArtifactBundleDigest`.
- **Implementado localmente:** rechazo de estrategias no implementadas.
- **Implementado localmente:** workflow stable/MSRV/features/Miri.
- **Implementado y contrastado externamente:** tres juegos de vectores SageMath
  10.7, regeneración determinista y verificación polinómica lenta.
- Ejecutar la matriz CI remota.
- Repetir Miri sobre la aritmética portable de H2; el runtime actual ya supera
  sus 9 tests aplicables.
- Crear un commit base revisable.

Salida: contratos externos congelados y matriz reproducible antes del primer
tipo GF(2²⁵⁶). El diagnóstico detallado está en
[`current-status-and-next.md`](current-status-and-next.md).

## H2 — Vertical `Gf2_256HhV1`

- Tipo transparente con limbs privados.
- Encoding y operadores.
- Producto ancho, reducción rápida/lenta y cuadrado propio.
- `mul_by_x`, potencia, inversa, Frobenius, traza y norma.
- Leyes genéricas y vectores Sage.

Salida: primer campo completo y portable.

## H3 — Generalización

- Generar `Gf2_128V1` y `Gf2_256AltV1`.
- Probar que el IR no codifica un módulo concreto.
- Compile-fail para confusión de tipos.

Salida: tres presentaciones nominalmente distintas.

## H4 — Batch portable

- ABI seguro de slices, catálogo estático y `EngineBuilder`.
- Operaciones out-of-place e in-place.
- Tests de tamaños, canarios, errores y cero asignaciones.

Salida: Fase 1 completa, todavía sin backends ISA.

## Gates

- MSRV 1.89.
- `no_std` sin dependencias obligatorias.
- `forbid(unsafe_code)`.
- Cero asignaciones en scalar/batch.
- Sin indirect calls escalares.
- Wrapper batch a menos de 3 % del kernel directo en lotes grandes.
