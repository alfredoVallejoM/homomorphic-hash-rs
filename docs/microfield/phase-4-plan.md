# Plan ejecutado de Fase 4 — campos primos

Fecha de cierre local: 2 de agosto de 2026.

## Objetivo y regla de diseño

La fase demuestra que `Field`, `Engine` y los algoritmos derivados no estaban
acoplados a característica dos. Se incorporan campos de característica prima
sin cambiar el layout ni la semántica de los campos binarios.

La separación autoritativa es:

```text
entero canónico ──encoding──► elemento público normalizado
                                  │
                                  ├─ producto ancho privado
                                  ├─ reducción privada certificada
                                  └─ estrategia batch seleccionada una vez
```

El algoritmo de representación y reducción pertenece a `ArtifactId`; el
módulo primo, el grado y el encoding pertenecen a `FieldId`.

## Hitos

| Hito | Entregable | Gate | Estado |
|---|---|---|---|
| F4.0 | Primo genérico congelado, identidad y ADR | búsqueda determinista y certificado reproducible | completado |
| F4.1 | `PrimeField`, `SquareRootField`, rangos y planes | API no filtra limbs ni Montgomery | completado |
| F4.2 | `Fp251V1` | aritmética exhaustiva y encoding de un byte | completado |
| F4.3 | `FpGoldilocks64V1` | Solinas y Barrett contra `%` independiente | completado |
| F4.4 | `Fp256GenericV1` | Montgomery CIOS portable de cuatro limbs | completado |
| F4.5 | Batch e ISA | AVX2 rentable; BMI2 radix-64 genérico y diferencial | completado |
| F4.6 | Certificados, bundles y Sage | replay interno y corpus externo determinista | completado |
| F4.7 | Calidad | `no_std`, Clippy, Miri, ASan y cero asignaciones | completado localmente; CI se valida tras push |
| F4.8 | Rendimiento y documentación | medición por estrategia y selección conservadora | completado |

## Campos congelados

| Tipo | Módulo | Representación privada | Reducción |
|---|---|---|---|
| `Fp251V1` | 251 | `u8` canónico | nativa de 16 bits |
| `FpGoldilocks64V1` | (2^{64}-2^{32}+1) | `u64` canónico | Barrett; Solinas como comparación |
| `Fp256GenericV1` | primo de 256 bits de ADR 0020 | cuatro `u64` Montgomery | CIOS radix (2^{64}) |

El primo genérico se obtiene de la semilla
`microfield:fp256-generic-v1:2026-08-02`. La búsqueda determinista encuentra el
primer primo válido en el intento 18. No se eligió una forma pseudo-Mersenne ni
otra estructura favorable.

## Trazabilidad de requisitos

- Contratos segregados: `PrimeField`, `SquareRootField`, `PrimeFieldSpec`
  interno y `PrimeWideProduct` interno.
- Estados de rango: `Reduced`, `Lazy2` y `Lazy4` son privados y no implementan
  `CanonicalEncoding`.
- Planes: `BarrettPlan`, `MontgomeryPlan`, `SolinasPlan` y `RangeContract`
  verifican forma y cotas sin entregar constantes privadas.
- Inversión: cada campo fija un `PrimeExponentiationPlan` distinto para
  (p-2); batch reutiliza exactamente el algoritmo de Fase 3.
- Primalidad: división completa para 251 y Pocklington para Goldilocks y el
  primo de 256 bits.
- Encoding: longitud exacta, little-endian, rechazo de enteros (ge p) y
  reducción solo mediante `from_bytes_mod_order`.
- Batch: los catálogos siguen sellados; `PrimeKernelMetadata` declara
  representación, reducción, rangos, lanes y packing.
- ISA: AVX2 procesa 32 residuos de `Fp251V1` por tile; el factory estático BMI2
  acepta cualquier tipo generado `VerifiedPrimeMontgomery64Field<N, 2N>` y lo
  encapsula en `VerifiedPrimeIsaStrategy`. La compatibilidad no lo selecciona
  automáticamente: cada campo
  necesita una calibración favorable y `Fp256GenericV1` perdió frente al
  portable en la CPU medida.
- IFMA: la representación permite una variante radix 52 futura, pero no se
  registra una implementación sin hardware, cobertura y ganancia medidas.

## Matriz de pruebas

- 63.001 pares exhaustivos para suma, resta y producto de `Fp251V1`;
- 20.000 pares Goldilocks reproducibles contra `u128`;
- cada bit de entradas de doble anchura y los 512 vectores base de REDC;
- límites `0`, `1`, `p-1`, rechazo de `p` y `p+1`;
- conversión Montgomery, constantes (R), (R^2) y
  (-p^{-1}\bmod 2^{64});
- Fermat, inversa, raíz cuadrada y raíz canónica mínima donde aplica;
- inversión batch con ceros en todos los tamaños normativos hasta 1024;
- tails e in-place ISA de 0 a 1024 elementos;
- producto `MULX` contra referencia para 64, 128, 192 y 256 bits, incluidos
  todos los pares de bits de base;
- aceptación de BMI2 forzado bajo `FixedSchedule`, sin promoción automática
  mientras la calibración no supere el umbral de rendimiento;
- compilación y ejecución de un tipo primo externo de un limb mediante la
  estrategia opaca, sin acceso a intrinsics ni catálogos construibles;
- corpus Sage de 24 casos con suma, resta, producto, square e inversa;
- bundles autenticados por ruta, longitud y SHA-256;
- cero asignaciones en rutas portables y seleccionadas.

## Decisiones aplazadas sin deuda oculta

La factory TOML v1 continúa aceptando solo campos binarios. La generación de
tipos primos externos pertenece a Fase 5: ampliar hoy el parser habría mezclado
un esquema estable con assurance probable/demostrada todavía no modelada.
También quedan fuera IFMA, un backend primo AArch64 preferido y Tonelli–Shanks
general. Ninguno es requisito para los tres campos mantenidos ni se anuncia
como disponible.

El cierre consolidado está en
[`phase-4-final-report.md`](phase-4-final-report.md).

## Extensión posterior F4.6-SIMD

Tras el cierre se ejecutó una ampliación SIMD sin reabrir los contratos
algebraicos. Añade Goldilocks AVX2 automático desde cuatro elementos, factories
AVX2 explícitos para primos externos canónicos de 8/16 bits y un desenrollado
de dos pares VPCLMUL. Fp251 conserva su implementación zero-copy porque el
bridge independiente del layout fue sustancialmente más lento. El plan y las
mediciones están en [`phase-4-6-plan.md`](phase-4-6-plan.md) y
[`phase-4-6-report.md`](phase-4-6-report.md).

## Extensión completada F4.7-PACKED-SIMD

El identificador no sustituye al hito histórico F4.7 de calidad de la tabla
anterior. Esta extensión convierte una vez campos externos a storage
persistente `u8`/`u16`/`u32` y ejecuta kernels directamente sobre lanes entre
operaciones. El ABI packed neutral no altera `KernelSet<F>` ordinario,
Fp251/Goldilocks conservan sus rutas directas y todo perfil externo permanece
explícito. Véanse [`phase-4-7-plan.md`](phase-4-7-plan.md) y el
[`informe final`](phase-4-7-final-report.md); ADR 0025 está aceptado.
