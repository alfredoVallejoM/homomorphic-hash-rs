# ADR 0022 — Backends primos y selección por evidencia

## Estado

Aceptada el 2 de agosto de 2026.

## Decisión

Se añaden `X86PrimeAvx2` y `X86PrimeBmi2` como slots internos del catálogo. La
capability detectada no basta: campo, metadata de rango y evidencia de
rendimiento deben coincidir antes de selección automática.

AVX2 para `Fp251V1` es automático desde 64 elementos. BMI2 para
`Fp256GenericV1` es correcto y forzable, pero no automático porque fue más
lento que el producto portable `u128`/CIOS en la CPU de referencia. Goldilocks
conserva portable hasta que un backend medido gane incluyendo reducción.

`PrimeKernelMetadata` autentica representación, reducción, rangos de entrada y
salida, lanes y necesidad de packing. `Engine` sigue seleccionando una vez y
ejecutando una llamada indirecta por lote.

## Seguridad

`backend/x86_prime.rs` es una frontera `unsafe` estrecha. Los entry points
seguros solo son registrables después de detección AVX2/BMI2; las cargas y
stores se acotan por tiles y los tails permanecen escalares. ASan, diferencial,
inventario SHA-256 y auditoría de instrucciones forman el gate conjunto.

## Consecuencias

Una optimización puede existir sin ser preferida. IFMA y AArch64 primo quedan
fuera hasta disponer de hardware, wrappers estables, pruebas de rango y una
región rentable reproducible. La API pública no cambia cuando aparezcan.
