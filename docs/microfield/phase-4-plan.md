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
| F4.5 | Batch e ISA | AVX2 rentable y BMI2 diferencial | completado |
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
- ISA: AVX2 procesa 32 residuos de `Fp251V1` por tile; BMI2 aporta un backend
  multi-limb verificable, pero no se selecciona automáticamente porque perdió
  frente al portable en la CPU medida.
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
