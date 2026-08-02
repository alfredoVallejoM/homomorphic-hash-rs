# Informe final de Fase 5

## Resultado

Microfield puede ahora recibir campos externos por dos rutas seguras:

1. generar un módulo Rust nominal, probado, revisable y optimizado por perfil;
2. construir un contexto binario o primo en runtime y procesar elementos o
   lotes homogéneos.

Ambas rutas comparten identidad matemática y pueden cruzarse mediante una
exportación que exige prueba y verifica el mismo `FieldId`.

## Entregables

- schema primo v1 estricto y cinco manifiestos de aceptación;
- `ValidationAssurance`, certificado Pocklington común y límites;
- factory estática `PrimeFieldFactory` y Builder;
- perfiles canónicos 8/16/32 y Montgomery radix-64 multi-limb;
- registro generado de los bridges AVX2/BMI2 existentes;
- bundle de diez archivos, `MicrofieldLock`, publicación transaccional y
  `PrimeArtifactCache` inmutable;
- cinco comandos CLI primos con salida humana/JSON;
- `DynField`, `DynElement`, almacenamiento inline/heap, `DynBatch` y
  `DynEngine`;
- puente `DynField::generate_static` para binarios y primos;
- benchmark Criterion dinámico contra estático;
- verificador Sage para bundles externos;
- suites adversariales, diferenciales y un crate consumidor generado durante
  test.

## Evidencia matemática

Los campos `u64` usan el conjunto de bases determinista completo. El campo de
256 bits de aceptación reejecuta un Pocklington con producto conocido de 135
bits, mayor que la raíz del módulo. Un contexto probable de Mersenne 127 se
valida pero su intento de generar Rust falla por contrato.

SageMath 10.7, bajo Conda `laboratorio_np` y caché `DOT_SAGE` aislada, verificó
16 vectores de suma, producto, cuadrado, inversa y encoding para:

- 65 521 (`u16`/AVX2 explícito);
- 4 294 967 291 (`u32`/AVX2 explícito);
- Goldilocks (Montgomery externo de un limb/BMI2 explícito);
- el primo genérico de 256 bits (Montgomery de cuatro limbs/BMI2 explícito).

## Seguridad y estabilidad

No se añadió `unsafe`. Los helpers Montgomery son safe, const-genéricos y
encapsulan carries/corrección branchless existentes. Un consumidor generado no
recibe intrinsics ni funciones raw. La caché rechaza entradas no regulares y
rutas relativas inseguras, autentica el lock desde `bundle.json` y comprueba
cada digest. Ocho publicadores simultáneos convergen en una única entrada
completa; una publicación fallida no mezcla versiones.

`num-bigint` está detrás de `generator`/`dynamic`; la matriz `no_std` y los
layouts mantenidos no cambian. El API dinámico permanece nominal: dos campos
del mismo tamaño no se pueden mezclar por accidente.

## Rendimiento

La generación estática elige la representación más estrecha compatible con
los bridges ya auditados. No promueve ISA externa sin calibración propia. El
contexto dinámico reutiliza su contexto y output batch, guarda hasta ocho
limbs inline y amortiza checks, pero conserva `GenericPortable`: sus
operaciones multiprecisión pueden asignar y no se describen como equivalentes
al tipo estático.

La ejecución Criterion local de cierre sobre GF(251) sitúa el batch dinámico
en torno a 19,1–19,8 millones de elementos/s para 64, 1 024 y 16 384
elementos. El batch estático alcanza aproximadamente 12,1–14,0 mil millones
de elementos/s en la misma máquina. Es una evidencia útil de la frontera de
coste —flexibilidad runtime frente a especialización—, no un umbral portable
ni una promesa contractual.

## Matriz de cierre ejecutada

- `cargo test -p microfield --all-features`: toda la unidad, integración y
  compile-fail de Microfield en verde;
- Fase 5: 10 tests dinámicos y 11 de generación/lock/caché/CLI/bridge;
- `cargo test -p microfield --all-features --all-targets`: además ejecuta todos
  los bancos Criterion como targets de prueba;
- cinco combinaciones mínimas comprobadas, incluido `--no-default-features`;
- Clippy de todos los features y targets con `-D warnings`;
- rustdoc con `-D warnings`, fmt y `git diff --check`;
- MSRV Rust 1.89: check integral y suites Fase 5;
- Miri: primalidad/Rabin, mezcla nominal y atomicidad de inversión batch;
- inventario `unsafe` v2 intacto: las cinco fronteras previas y ninguna nueva;
- consumidor externo compilado sin features por defecto;
- SageMath 10.7: cuatro perfiles, 16 vectores exactos por perfil;
- paquete legado: 447 de 447 tests de biblioteca en verde.

El `cargo test --workspace` global sigue intentando compilar el ejemplo legado
`chemistry_paper.rs` y falla en su uso preexistente de `Response::body_mut`.
No pertenece a Fase 5 ni ha sido ocultado: su corrección forma parte del
inventario/migración del legado planificado para Fase 6.

## Límites deliberados

- no hay JIT ni plugins runtime;
- no hay catálogo ISA dinámico en esta fase;
- los artefactos son fuente target-neutral, no objetos por triple;
- una prueba probable no se eleva implícitamente;
- el oráculo Sage es un gate de certificación, no dependencia runtime;
- benchmarks publican comparaciones, no una promesa universal de ratio.

## Continuación

La siguiente fase es la Fase 6 corregida del roadmap: inventario y congelación
del legado, migración sobre `microfield`, identidades de encoder/firma y el
track exacto de canonización de grafos. Ninguna de esas capas se añadirá a
`field`, `kernel` o `dynamic`.
