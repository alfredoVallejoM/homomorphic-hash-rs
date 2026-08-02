# ADR 0022 — Backends primos y selección por evidencia

## Estado

Aceptada el 2 de agosto de 2026.

## Decisión

Se añaden `X86PrimeAvx2` y `X86PrimeBmi2` como slots internos del catálogo. La
capability detectada no basta: campo, metadata de rango y evidencia de
rendimiento deben coincidir antes de selección automática.

AVX2 para `Fp251V1` es automático desde 64 elementos. BMI2 para
`Fp256GenericV1` es correcto y forzable, pero no automático porque fue más
lento que el producto portable `u128`/CIOS en la CPU de referencia. La
extensión registrada en ADR 0024 incorpora Goldilocks AVX2 automático desde
cuatro elementos tras medir producto, square y suma incluyendo reducción.

La corrección de cierre queda desarrollada en ADR 0023: BMI2 usa un factory
estático genérico para cualquier representación Montgomery radix 64, pero
compatibilidad estructural y promoción automática permanecen separadas.
ADR 0024 aplica la misma separación a los perfiles AVX2 canónicos `u8`/`u16`.

`PrimeKernelMetadata` autentica representación, reducción, rangos de entrada y
salida, lanes y necesidad de packing. `Engine` sigue seleccionando una vez y
ejecutando una llamada indirecta por lote.

## Seguridad

`backend/x86_prime.rs` es una frontera `unsafe` estrecha. Los entry points
seguros solo son registrables después de detección AVX2/BMI2; las cargas y
stores se acotan por tiles y los tails permanecen escalares. ASan, diferencial,
inventario SHA-256 y auditoría de instrucciones forman el gate conjunto.

ADR 0023 sustituye las cadenas variables por recorridos completos y selección
sin ramas. BMI2 publica por ello `ScheduleKind::Fixed` y `FixedSchedule` puede
forzarlo. No se deriva una afirmación integral de constant-time únicamente a
partir de esta propiedad ni de `MULX`.

## Consecuencias

Una optimización puede existir sin ser preferida. IFMA y AArch64 primo quedan
fuera hasta disponer de hardware, wrappers estables, pruebas de rango y una
región rentable reproducible. La API pública no cambia cuando aparezcan.
