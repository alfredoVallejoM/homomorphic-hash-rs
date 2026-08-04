# Extensión F4.6-SIMD — plan ejecutado

Fecha: 2 de agosto de 2026.

Esta extensión posterior al cierre original de Fase 4 amplía SIMD sin cambiar
la API algebraica ni confundir compatibilidad ISA con rentabilidad. No sustituye
el hito histórico F4.6 de certificados: adopta el nombre solicitado
`F4.6-SIMD` para dejar clara su trazabilidad.

## Objetivo

1. vectorizar más campos mantenidos donde el layout y la reducción lo permiten;
2. ofrecer factories estáticos reutilizables para tipos generados externos;
3. retirar ramas de operación del bucle vectorial;
4. mejorar el throughput binario VPCLMUL sin promoverlo sin evidencia;
5. conservar selección previa, cero heap y una única llamada indirecta por lote.

## Criterio de generalización

La generalización se hace por **perfil aritmético y representación**, no solo
por tamaño:

| Perfil | Tesela x86-64 | Estado de selección |
|---|---:|---|
| primo canónico `u8`, `3 <= p <= 251`, Barrett | 32 residuos AVX2 | candidato externo explícito |
| primo canónico `u16`, `3 <= p <= 65_521`, Barrett | 16 residuos AVX2 | candidato externo explícito |
| Goldilocks canónico `u64`, reducción Solinas | 4 residuos AVX2 | automático desde 4 |
| Montgomery radix 64 de `N` limbs | producto BMI2 `N x N` | candidato explícito |
| binario ABI 3 compatible | dos elementos por operación VPCLMUL | candidato explícito |

Un perfil externo es estructuralmente elegible, pero no posee por ello una
calibración atribuible a su campo, compilador y CPU. Sus estrategias SIMD se
mantienen forzables y fuera de `Auto`.

## Hitos y gates

| Hito | Entregable | Gate | Estado |
|---|---|---|---|
| F4.6-S0 | línea base por operación y tamaño | portable y backend forzado medidos por separado | completado |
| F4.6-S1 | puente AVX2 canónico `u8`/`u16` | exhaustivo Fp17, diferencial Fp65521, tails y cero alloc | completado |
| F4.6-S2 | Goldilocks AVX2 | diferencial de producto ancho, fronteras, in-place y selector | completado |
| F4.6-S3 | VPCLMUL desenrollado | presets y ABI 3 coinciden con portable; sin cambio de política | completado |
| F4.6-S4 | especialización estática | suma/producto/square sin enum de operación en el loop | completado |
| F4.6-S5 | seguridad y ensamblado | ASan, inventario SHA-256 e instrucciones requeridas | completado localmente |
| F4.6-S6 | documentación y cierre | ADR, informe, contratos y roadmap coherentes | completado |

## Diseño de coste cero

`VerifiedPrimeSimd8Strategy<F>` y `VerifiedPrimeSimd16Strategy<F>` son factories
estáticos opacos. El código generado aporta módulo, recíproca y conversiones
seguras; Microfield conserva intrinsics, Barrett, tails, punteros de función y
metadata. Los parámetros const de operación producen monomorfizaciones
separadas para suma, producto y square.

La ruta mantenida de `Fp251V1` sigue especializada y trabaja directamente sobre
su newtype transparente. Sustituirla por el puente independiente del layout
añadiría extracción y compactación de valores; la medición local observó una
regresión cercana a 8x en 64 elementos. El puente general resuelve extensión y
seguridad, no reemplaza una ruta zero-copy más rápida.

Goldilocks sí recibe una implementación específica: cuatro productos `u64 x
u64` se descomponen en productos de 32 bits, se reconstruyen los high/low de
128 bits y se reducen con cuatro folds fijos usando
`2^64 = 2^32 - 1 (mod p)`. Comparaciones unsigned y corrección final son
branchless.

## Definición de terminado

- ninguna representación SIMD es pública;
- no hay detección de CPU ni asignación dentro del kernel;
- tails `0..tile-1`, tamaños frontera e in-place coinciden con portable;
- salida transaccional se conserva ante longitudes inválidas;
- los perfiles externos nunca entran automáticamente sin calibración propia;
- un backend mantenido solo entra en `Auto` si no degrada más del 3 % su región;
- Clippy, rustdoc, feature matrix, MSRV, ASan y auditorías estructurales quedan
  verdes.

El resultado medido y los límites se detallan en
[`phase-4-6-report.md`](phase-4-6-report.md).
