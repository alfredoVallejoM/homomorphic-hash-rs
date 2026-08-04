# Estado actual y siguiente plan

Fecha de revisión: 4 de agosto de 2026.

## Diagnóstico ejecutivo

F6.0–F6.8 están completados localmente. La aritmética legacy de
`GaloisSignature256` delega en `microfield`; `structural` implementa firmas
aditivas, secuenciales, bidireccionales y de multiconjunto simple/multievaluado
con identidad completa, contadores, factores cero, wire canónico y operaciones
masivas transaccionales. La API cubre tipos mantenidos, perfiles externos
generados y contextos runtime detrás de `dynamic-fields`; estático y dinámico
producen el mismo wire cuando el campo y parámetros son iguales. Las antiguas
“pruebas de inclusión” quedan correctamente clasificadas como residuos
algebraicos. El canonizador histórico y sus heurísticas continúan congelados.
La discusión F6.G0 ya ha cerrado y F6.G0–G2 implementan ahora
un modelo CSR por incidencias, etiquetado rápido genérico, perfiles F251 y
externos, huella híbrida SHA-256 y salida exacta únicamente para particiones
discretas. Véanse
[`phase-6-legacy-audit.md`](phase-6-legacy-audit.md),
[`phase-6-pre-canon-plan.md`](phase-6-pre-canon-plan.md) y el
[`informe pre-canon`](phase-6-pre-canon-final-report.md), junto con
[`phase-6-fast-graph.md`](phase-6-fast-graph.md).

Actualización de Fase 4: la fase de campos primos está cerrada localmente.
`Fp251V1`, `FpGoldilocks64V1` y `Fp256GenericV1` están certificados, poseen
encoding canónico estricto y funcionan con los algoritmos y `Engine` existentes.
El portable incluye Native, Barrett, Solinas y Montgomery CIOS. La extensión
F4.6-SIMD mantiene AVX2 zero-copy para 251 desde 64 elementos, añade Goldilocks
AVX2 automático desde 4 y ofrece bridges AVX2 explícitos para primos externos
canónicos de 8 y 16 bits. BMI2 se generalizó a cualquier contrato
Montgomery radix 64, pero la instancia de 256 bits es forzable y no automática
porque la medición no justificó promoverla. Carry y corrección se reescribieron
con iteraciones fijas y selección branchless; BMI2 publica ahora `Fixed` y pasa
una auditoría ASM específica. El corpus Sage, bundles, cero
asignaciones, `no_std`, Miri/ASan y auditoría ASM forman el cierre. El detalle
está en [`phase-4-final-report.md`](phase-4-final-report.md) y la ampliación
SIMD en [`phase-4-6-report.md`](phase-4-6-report.md).

F4.7-PACKED-SIMD está completada localmente. Separa el layout lógico `F` del
storage persistente `u8`/`u16`/`u32`: los bridges externos convierten una vez,
ejecutan pipelines de cinco operaciones sin repacking y convierten de vuelta
solo al salir. El bridge `u32` queda correcto y explícito; la evidencia de
aceleración publicada corresponde a `u16`, que supera conservadoramente el
58 % desde 64 elementos en la máquina calibrada. El cierre y sus límites están
en [`phase-4-7-final-report.md`](phase-4-7-final-report.md).

Actualización de Fase 3: los algoritmos derivados están implementados y
cerrados localmente. Existen inversión batch tolerante a cero, máscara compacta,
workspace tipado, scans, las dos orientaciones de Horner, `mul_add_into` y
potencias fijas. El planner usa schema 2/IR v4 y verifica simbólicamente la
cadena Itoh–Tsujii exacta antes de calcular artefactos. La ruta prestada pasa
el gate de cero asignaciones y no amplía `unsafe`.

La planificación Fases 3–7 se corrigió en
[`phases-3-7-roadmap.md`](phases-3-7-roadmap.md). La parte algebraica y la
migración legacy de Fase 6 están cerradas. El track de grafos G0–G7 queda como
baseline v0. F6.G8 ya separa `GraphSchemaId` de
`GraphAnalysisProfileId`, introduce encoding/parser v1, mappings verificados y
la fachada fail-closed `Microcanon`. El baseline G9 ejecuta refinamiento exacto,
componentes e IR exhaustivo sin depender de campos; el adapter histórico delega
en él. G10 añade la estrategia compacta predeterminada, IDs internados, arena
plana, trazas, automorfismos verificados, poda por órbitas/prefijo y presupuestos
de bytes, profundidad y tiempo. Conserva G9 como referencia diferencial y
produce los mismos bytes. G11 añade ya assurance, lanes independientes,
secuencias multievaluadas, moments, patterns L0–L3, matrix RG1 y theta RG2. Su
bundle Goldilocks no colisiona en las 12.346 clases n=8, pero CFI sigue
indistinguible y conserva al exacto como autoridad. La extensión final G11
añade policy estática/dinámica por característica y closed walks de longitud
`u64` mediante recurrencias exactas de campo. G12 sustituye la doble
canonización por prefiltros exactos, block-cut iterativo, bosques exactos sin
recursión, refinamiento conjunto y matcher fail-first. CFI(K4) se decide por
agotamiento exacto en 6.976 asignaciones. G13/G14 cierran ahora el pipeline
adaptativo, el 2-WL localizado y la incrementalidad transaccional. G15 queda
planificado como gate de consumo interno —API y protocolos de firmas/campos,
persistencia, aplicaciones, grafos/DAG, oráculos, robustez y SLO— y la
publicación pasa a una fase posterior separada. F6.V1–V6
conserva valor como caracterización reproducible
del baseline. El diseño y la entrega están en
[`phase-6-canonization-v2-plan.md`](phase-6-canonization-v2-plan.md) y
[`ADR 0031`](adr/0031-certified-canonization-core.md), con evidencia en
[`phase-6-g8-g9-implementation-report.md`](phase-6-g8-g9-implementation-report.md).
El cierre y las mediciones de G10 están en
[`phase-6-g10-final-report.md`](phase-6-g10-final-report.md).
El cierre, holdout y límites de G11 están en
[`phase-6-g11-final-report.md`](phase-6-g11-final-report.md).
El cierre exacto y las mediciones pareadas están en
[`phase-6-g12-final-report.md`](phase-6-g12-final-report.md).

La Fase 2 está cerrada. H2.8 transforma la calibración, seguridad y
compatibilidad en contratos versionados: tabla de selección v1 compilada como
constantes, corpus diferencial persistente, inventario SHA-256 de `unsafe` y
matriz runtime/codegen. PCLMUL permanece automático; VPCLMUL y PMULL son
correctos y forzables, pero siguen `explicit_only` porque no existe todavía
evidencia favorable en dos familias de CPU. Disponibilidad, corrección y
rendimiento permanecen separados.

La Fase 1 completa, H0–H4, está integrada en `origin/main`. H4 entró por
fast-forward mediante `1f176ab`; el `main` resultante superó sus cinco jobs en
[`30703842091`](https://github.com/alfredoVallejoM/homomorphic-hash-rs/actions/runs/30703842091).
`EngineBuilder<F>` selecciona un catálogo estático sellado; `Engine<F>` valida
una vez y delega mediante una sola llamada por lote. El backend portable ofrece
suma, producto y cuadrado out-of-place, y producto/cuadrado in-place, sin
`unsafe`, heap, packing, detección de CPU ni paralelismo.

| Área | Estado | Evidencia |
|---|---|---|
| Fase 1 | Cerrada en `main` | `1f176ab`; cinco jobs verdes en `30703842091` |
| Fase 2 | Cerrada conservadoramente | H2.1–H2.8; informe final y tabla de selección v1 |
| Fase 3 | Cerrada y publicada | algoritmos derivados, IR v4, Miri/ASan y benchmark |
| Fase 4 | Cerrada, corregida y ampliada | tres primos, certificados, Sage, Goldilocks/Fp251 AVX2 y bridges SIMD/BMI2 genéricos |
| F4.7-PACKED-SIMD | Completada localmente | ABI packed por lanes, storage persistente, SIMD `u8`/`u16`, candidato `u32` y calibración adversaria |
| Fase 5 | Cerrada localmente | factory prima, assurance, bundles, contextos dinámicos y puente estático |
| F6.0–F6.8 | Completada localmente | auditoría legacy, cinco firmas, campos generados/runtime, identidades, tracking, residual y wire schema 1 |
| F6.G0–G2 | Completada localmente | CSR relacional, motor lineal multi-campo, F251, SHA-256 invariante y salida discreta exacta |
| F6.G3 | Completo localmente | preparación/workspace, batch AVX2 medido, paralelismo determinista y migración del canonizador legado |
| F6.G4–G7 | Baseline v0 completado | incrementalidad, degeneración, exacto básico por componentes, perfil global v2 y corpus externo; no es el cierre estable |
| F6.V1–V6 | Implementada; evidencia de publicación aún bloqueante | 145.636 leyes, 63.232 reconciliaciones, 12.346 grafos n=8, adversariales, verticales y runners x86/ARM |
| F6.G8 | Completado localmente | schema/profile separados, encoding/parser v1, key, mappings y verifier |
| F6.G9 | Baseline completado localmente | Microcanon independiente e IR exhaustivo; 32.768 grafos n=6 en 156 clases exactas |
| F6.G10 | Completado localmente | arena plana/IDs, G9 diferencial, órbitas y prefijo certificados, workspace y budgets; 92,8 % menos nodos en C32 |
| F6.G11 / F6.RG | Cerrado localmente | assurance, campos externos/runtime, histograma de grado + multiset, patterns, producto, matrix/theta y closed walks `u64`; homomorfismos/resolventes quedan como investigación |
| F6.G12 | Cerrado localmente | block-cut, bosques exactos, matcher pareado, mapping verificado, CFI y benchmark frente a doble canonización |
| F6.G13–G14 | Cerrado localmente | pipeline de seis niveles, 2-WL localizado, `GraphDelta`, replay/fallback y campaña determinista |
| F6.G15 | Planificado | APIs de firmas/campos, protocolos y persistencia; aplicaciones, grafos/DAG, oráculos, fuzzing, SLO y go/no-go |
| API algebraica | Correcto | `F2` y tres campos completos, nominales y monomorfizados |
| Batch H4 | Integrado | catálogo, builder, fachada y backend portable en `main` |
| Errores batch | Transaccional | todas las longitudes se validan antes de escribir |
| `no_std` | Correcto | ABI 3 scalar-only sin `portable`; batch sin `std` ni `alloc` |
| Asignaciones | Correcto local y remoto | contador externo: cero en las cinco operaciones y tres campos |
| Dispatch | Correcto en ensamblado | dos comparaciones y una llamada indirecta por operación batch |
| MSRV | Correcto en H4 | Rust 1.89, incluida la medición de cero asignaciones |
| Miri portable/ABI 3 | Correcto local | suite portable completa y 11 tests del consumidor externo |
| Rendimiento H4 | Gate local superado | peor sobrecoste observado: 1,9 % en producto HH/4096 |
| ISA x86-64 | PCLMUL implementado | tres presets; producto, square, tails e in-place |
| Factory H2.1 | Implementada | tipos externos nominales 2..=4096, Rabin y emisión atómica |
| Optimizador H2.2 | Implementado | tres reducciones, square dedicado e Itoh–Tsujii |
| ABI de codegen | Compatible 1..=3 | constante única emite v3; matriz versionada conserva v1/v2 |
| Capabilities H2.3 | Implementado | snapshot no falsificable, detección x86-64/AArch64 y `portable_only` |
| Selector H2.3 | Implementado | cinco políticas y errores separados por build/campo/CPU/política |
| Perfiles externos | Implementados | grados 9, 10, 128, 192 y 233; tres clases y tres reducciones |
| ISA x86-64 | PCLMUL activo | presets automáticos medidos; perfiles externos explícitos |
| ISA AArch64 | PMULL explícito | presets y ABI 3; calibración nativa pendiente |
| Packed H2.6 | Implementado | owned `alloc`, vistas sin `alloc`, plan sellado y operaciones in-place |
| ISA x86-64 H2.7 | VPCLMUL explícito | presets y ABI 3, pares, tails, in-place y `vzeroupper` |
| Packing H2.7 | `Aos` + `AosLanePairs` | layout sellado, padding par y alineación 32 para VPCLMUL |
| Frontera `unsafe` | Confinada y autenticada | `deny` global, cinco hashes revisados y test estructural |
| ASan multi-ISA | Correcto local | presets y 11 tests externos en x86-64/AArch64 |
| PMULL QEMU | Correcto | 3 tests mantenidos + 11 externos, también bajo ASan |
| PMULL hardware | Correcto funcional | job ARM64 real verde en `30716211486`; calibración pendiente |
| Calibración H2.8 | Tabla v1 correcta | 9 decisiones; captura Criterion x86/ARM; promoción multi-familia |
| Corpus H2.8 | Persistente | 20 seeds/tamaños con reproducción hasta el primer índice divergente |

## Decisiones H4/H2.3/H2.4 materializadas

1. `kernel` posee el ABI neutral y metadatos, no implementaciones matemáticas.
2. `backend::portable` contiene los bucles seguros y sin asignación.
3. Cada preset registra su `KernelCatalog<F>` estático; ABI 1/2 recibe portable
   y ABI 3 adjunta un perfil ISA verificado y explícito.
4. `BuiltinField` es público para bounds genéricos, oculto y sellado para
   impedir catálogos externos inseguros.
5. `EngineBuilder` conserva política, backend forzado, tamaño esperado y una
   instantánea de capabilities; la selección ocurre una vez.
6. `Engine` es inmutable, `Copy + Send + Sync` y solo almacena referencia a la
   estrategia, política y hint de tamaño.
7. `FixedSchedule` falla de forma tipada: el producto portable actual depende
   de los operandos y no recibe una garantía falsa.
8. `build()` nunca detecta implícitamente; `detect()` consulta la CPU una vez
   con `std`; `portable_only` cubre `no_std`.
9. Un backend forzado diferencia `BackendNotCompiled`,
   `BackendUnsupportedByField`, `BackendUnsupportedByCpu` y política.
10. PCLMUL y VPCLMUL se registran en x86-64 y PMULL en AArch64. Los adapters
    genéricos aceptan los tres backends mediante ABI 3.
11. `automatic_selection` separa corrección de calibración: PCLMUL preset puede
    entrar en `Auto`; PMULL, VPCLMUL y perfiles externos requieren
    `force_backend`.
12. El perfil autentica layout, producto, reducción, backends y schedule. La
    clasificación completa es fija en low-tail y dependiente de datos en
    sparse/dense, de modo que `FixedSchedule` no acepta una promesa falsa.
13. `PackingPlan` solo lo construye `Engine`; fija backend, campo, layout,
    longitud, padding y alineamiento. `PackedBatch` owned y las vistas usan la
    misma operación de kernel sin repacking oculto.
14. `AosLanePairs` se asocia únicamente a VPCLMUL: conserva AoS dentro de una
    tesela de dos, padding par y alineación 32. Ningún caller construye planes.
15. VPCLMUL queda fuera de selección automática según medición: una ganancia
    pequeña GF(2¹²⁸) no compensa la regresión clara de 256 bits ni permite una
    regla universal por feature bits.

La frontera se registra en
[`ADR 0009`](adr/0009-portable-batch-engine.md).
La detección y el selector se fijan en
[`ADR 0012`](adr/0012-cpu-capabilities-and-static-selector.md).
El backend x86-64 y su frontera de seguridad se fijan en
[`ADR 0013`](adr/0013-x86-pclmul-backend.md).
El puente externo y PMULL se fijan en
[`ADR 0014`](adr/0014-verified-external-isa-profiles.md) y
[`ADR 0015`](adr/0015-aarch64-pmull-backend.md). H2.6 se fija en
[`ADR 0016`](adr/0016-persistent-packed-batches.md) y H2.7 en
[`ADR 0017`](adr/0017-x86-vpclmul-lane-pairs.md).

## Cobertura

La suite raíz x86-64 de Microfield contiene 164 tests de runtime y siete
doctests compile-fail; el feature de conteo añade dos tests. El target AArch64
añade tres tests PMULL específicos. El consumidor generado añade 11 tests de
runtime y dos compile-fail; la integración legada conserva tres tests. H4
añadió:

- equivalencia batch/escalar para los tres campos;
- tamaños `0, 1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 255, 256, 1024,
  16384`;
- suma, producto y cuadrado out-of-place;
- producto y cuadrado in-place;
- canarios antes y después de todos los buffers de salida;
- error exacto y salida intacta para cada forma de longitud incompatible;
- slices vacíos válidos;
- metadatos, políticas, catálogo compartido y backend forzado;
- compile-fail que impide acceder a `KernelSet` o construir catálogos;
- contador aislado que observa cero asignaciones y cero bytes asignados;
- compilación `builtin-fields` sin activar el motor portable.

La compilación `no_std` sin `alloc` proporciona una barrera estructural y el
feature opcional `count-allocations` añade medición dinámica sin introducir
`unsafe` en Microfield. El contador se activa solo en el gate de pruebas.

H2.2 añade comparación diferencial low-tail/sparse/dense, la matriz
64..=4096 y el fixture denso GF(2¹⁰). ABI 3 mantiene compatibilidad runtime
1..=3. Miri ejecuta helpers internos, presets y los cinco campos externos
generados.

H2.3 añade 13 tests de runtime: tabla forzada exhaustiva de 491.520
combinaciones, matriz automática, requisitos de features por arquitectura,
detección contra los macros de Rust, fallback conservador, diagnóstico exacto,
cero asignaciones, invariantes de metadata del catálogo y construcción
concurrente determinista. El consumidor externo ABI 2 prueba además que el
método nuevo con implementación por defecto no rompe su código generado.

H2.4 añade dos oráculos internos y cuatro gates de integración. Karatsuba se
compara con schoolbook PCLMUL en casos frontera y 4096 muestras; el backend
público se compara contra portable para los tres campos y 17 tamaños, con
todos los bits de la base, densos, patrones alternos, canarios, in-place y errores
transaccionales. La misma suite pública pasa bajo AddressSanitizer.

El puente ABI 3 prueba grados externos 9 sparse, 10 dense, 128 low-tail, 192
alineado no potencia de dos y 233 no alineado. Recalcula el digest del perfil
de forma independiente, conserva
`Auto = Portable`, exige detección al forzar ISA y compara producto, cuadrado e
in-place con el oráculo portable en x86-64 y AArch64. También comprueba que
`FixedSchedule` acepta únicamente el schedule autenticado como fijo.

H2.5 replica la suite normativa para PMULL: todos los bits de base, 17 tamaños
hasta 16 384, tres presets, canarios, in-place y errores transaccionales. QEMU
8.2 `-cpu max` ejecuta los tres tests específicos y los 11 externos; ambos
conjuntos pasan además bajo AddressSanitizer. El test de alcance recorre `src`
y, tras Fase 4, permite exactamente cinco excepciones `unsafe`: cuatro
adapters ISA y el único módulo de storage alineado.

H2.6 añade cuatro tests owned, cinco tests de vistas y dos unit tests de
planner/storage. Cubre los tres presets, backend ISA detectado, cinco perfiles
externos, longitud cero, tails, todos los offsets de alineamiento, overflow,
`Send + Sync`, planes/backend incompatibles, atomicidad y rutas in-place. Dos
doctests impiden aliasing y serialización; el contador confirma cero
asignaciones en operaciones owned reutilizadas y en toda la ruta de vistas.

H2.7 añade cuatro gates públicos VPCLMUL. Recorren los tres presets, todos los
bits de base y 20 tamaños con fronteras pares/impares hasta 16 384; comparan
portable, PCLMUL y VPCLMUL, canarios, tails, in-place, errores transaccionales,
owned, vistas, 32 offsets de alineamiento y padding. Los cinco perfiles externos
cubren VPCLMUL sobre sparse, dense y low-tail. ASan ejecuta ambas fronteras x86,
el contador confirma cero asignaciones y la auditoría exige `vpclmul*` más
`vzeroupper` sin dispatch interno ni asignador.

La matriz CI añade `portable` sin presets, `std + portable`, auditoría de
ensamblado cruzada y un job nativo `ubuntu-24.04-arm` con diferencial PMULL,
perfiles externos, asignaciones, Clippy y ASan. El run
[`30716211486`](https://github.com/alfredoVallejoM/homomorphic-hash-rs/actions/runs/30716211486)
pasó sobre hardware ARM64 real; localmente AArch64 también se compila y se usó
QEMU como apoyo funcional. La selección automática espera todavía una medición
Criterion nativa representativa. SageMath 10.7 bajo `laboratorio_np` regeneró
los tres vectores mantenidos con diff byte a byte vacío.

## Rendimiento H4

Criterion, Rust 1.97.1, release, Linux x86-64, Intel Core i7-13700HX, lotes de
4096 elementos:

| Campo/operación | Bucle directo | `Engine` | Diferencia observada |
|---|---:|---:|---:|
| GF(2¹²⁸) producto | 456,23 µs | 448,55 µs | -1,7 % |
| GF(2¹²⁸) suma | 1,589 µs | 1,587 µs | -0,1 % |
| HH-256 producto | 1,8190 ms | 1,8533 ms | +1,9 % |
| HH-256 suma | 4,294 µs | 4,296 µs | +0,04 % |
| Alt-256 producto | 1,9702 ms | 1,8395 ms | -6,6 % |
| Alt-256 suma | 4,302 µs | 4,295 µs | -0,1 % |

El peor sobrecoste positivo queda por debajo del gate de 3 %. Las diferencias
favorables, especialmente Alt-256, se consideran ruido/variación de código y
no una mejora atribuible a la fachada.

El desensamblado del producto HH muestra dos comparaciones de longitud antes de
`call *0x8(%rax)`: una única llamada indirecta al kernel por invocación. El
kernel contiene una llamada directa al producto ancho y ninguna indirección o
referencia al asignador.

H2.3 no añade campos a `Engine` ni modifica sus operaciones. Criterion mide la
construcción con capabilities portables en 852,29–868,25 ps y con detección
cacheada en 1,0558–1,0602 ns en la máquina local. El contador de asignaciones
observa cero en ambas rutas. El detalle y el comando reproducible están en el
[README de benchmarks](../../crates/microfield/benches/README.md).

## Rendimiento H2.4

Criterion, Rust 1.97.1/LLVM 22.1.6, release, Linux x86-64, Intel Core
i7-13700HX. Intervalos rápidos usados para decidir elegibilidad:

| Campo/lote/operación | Portable `Engine` | PCLMUL `Engine` |
|---|---:|---:|
| GF(2¹²⁸)/1 producto | 89,798–89,928 ns | 4,7955–4,9803 ns |
| GF(2¹²⁸)/1 cuadrado | 6,5390–6,6661 ns | 4,1649–4,2017 ns |
| HH-256/1 producto | 376,41–380,92 ns | 11,245–11,265 ns |
| HH-256/1 cuadrado | 10,858–10,932 ns | 7,9176–8,1900 ns |
| Alt-256/1 producto | 359,07–361,97 ns | 11,269–11,368 ns |
| Alt-256/1 cuadrado | 10,749–10,776 ns | 7,9207–7,9840 ns |
| HH-256/4096 producto | 1,4687–1,4811 ms | 39,055–39,333 µs |

Incluso comparando el peor PCLMUL con el mejor portable, el cuadrado mejora
35,7 %, 24,6 % y 25,7 % desde un elemento. Producto supera ampliamente el
gate de 20 % y HH-256/4096 mejora aproximadamente 37,5x. Por ello los tres
catálogos publican `minimum_batch = 1`. El audit automático encuentra
`pclmullqlqdq` y no encuentra asignador ni llamada indirecta dentro del kernel.
Estas cifras orientan el selector en esta CPU; no son una garantía universal.

## Rendimiento H2.5

No se publica todavía una cifra PMULL. QEMU demuestra corrección, ASan y
portabilidad del binario, pero no modela el rendimiento de un núcleo ARM real.
Por eso todos los catálogos PMULL conservan
`automatic_selection = false`; el benchmark separa la ruta forzada para que la
calibración nativa pueda medir kernel, fachada y dispatch sin alterar la API.

## Hallazgos abiertos

### Media — Alcance de «transaccional»

La publicación de artefactos no promete durabilidad frente a caída del sistema,
publicación concurrente ni atomicidad entre filesystems. Requerirá una política
de coordinación antes de uso concurrente.

### Fuera de alcance

Los auto-benches históricos inválidos se han retirado de los targets
mantenidos sin borrar su fuente. El canonizador, Bloom y spectral históricos se
mantienen como legado experimental; el motor nuevo no incorpora índices de
entrada ni expansión de hiperaristas a cliques. La ausencia observada de
colisiones no constituye evidencia de canonización.

## Siguiente orden

### Cierre de Fase 1

La matriz local y la remota están cerradas: stable, Clippy, rustdoc, features,
MSRV 1.89, Miri, artefactos deterministas y las 447 pruebas de la biblioteca
legada han terminado correctamente. H4 está integrado y no queda trabajo
pendiente dentro del alcance de Fase 1.

Salida: Fase 1 portable completa.

El detalle consolidado está en
[`phase-1-final-report.md`](phase-1-final-report.md).

### Fase 2 cerrada

H2.1 ha materializado `BinaryFieldFactory`: un consumidor puede declarar
GF(2^m), validarlo y generar en `build.rs` un tipo nominal con scalar y batch
portable, sin editar Microfield. H2.2 añade el optimizador portable estático y
mantiene v1 como oráculo diferencial. H2.3 añade capabilities confiables,
catálogo ampliado y selector inmutable. H2.4 añade PCLMUL. El puente ABI 3
extiende perfiles verificados a campos externos y H2.5 añade PMULL en AArch64.
H2.6 añade batches persistentes owned/prestados y almacenamiento alineado. H2.7
añade VPCLMUL y `AosLanePairs` sin degradar la selección automática. H2.8
cierra calibración, seguridad, reproducibilidad y estabilidad runtime/codegen.
La evidencia insuficiente se representa como selección explícita, nunca como
un threshold optimista. El informe consolidado está en
[`phase-2-final-report.md`](phase-2-final-report.md).

### F6.V1–V6 implementada; núcleo de grafos reabierto

La generación prima externa, assurance probado/probable, bundle/lock, caché,
CLI, contextos dinámicos y puente dinámico→estático están implementados sin
alterar tipos mantenidos ni dispatch escalar. Los cuatro perfiles de aceptación
compilan en un consumidor y SageMath 10.7 valida sus vectores.

F6.0–F6.8 inventariaron, generalizaron y migraron la parte algebraica del legado.
F6.G0–G4 añaden modelo exacto, etiquetador rápido, preparación/workspaces,
SoA/AVX2 explícito, paralelismo determinista e incrementalidad. F6.G5 separa
aliasing de campo y degeneración local, identifica evidencia multi-campo y
valida familias regulares con un oráculo exhaustivo y SageMath. F6.G6 incorpora
canonización exacta opt-in con presupuesto de nodos y estado retenido,
publicando `BudgetExhausted` sin candidato cuando no completa el árbol. Los
límites prácticos detectados obligaron a F6.G7: `analyze_discriminating` añade
un descriptor global exacto y motivos acotados; la canonización se descompone
por componentes. El corpus fijado valida 1.253 clases del Graph Atlas, 188
moléculas MUTAG, email-Eu-core y el hipergrafo diseasome. Los cierres están en
[`phase-6-g3-final-report.md`](phase-6-g3-final-report.md) y
[`phase-6-g4-final-report.md`](phase-6-g4-final-report.md), con el informe final
en [`phase-6-g5-g6-final-report.md`](phase-6-g5-g6-final-report.md).
El cierre acumulado está en
[`phase-6-final-report.md`](phase-6-final-report.md).
La corrección v2 está en
[`phase-6-g7-final-report.md`](phase-6-g7-final-report.md).

[`F6.V`](phase-6-validation-plan.md) ya dispone de laboratorio y primera
campaña. El resultado y los claims permitidos/prohibidos están en
[`phase-6-validation-final-report.md`](phase-6-validation-final-report.md). La
auditoría posterior sí amplía y corrige el núcleo: G8–G15 separan canonización y
firmas, añaden mappings verificados, podas demostradas, comparación pareada y
presupuesto integral. En paralelo,
[`F6.RG`](relational-green-invariant-research.md) estudiará una jerarquía de
Green inspirada en `Theta` sin presentarla como completa ni novedosa antes de la
baseline bibliográfica. FFT, torres, estabilización, licencia y publicación
permanecen bloqueadas hasta cerrar esta frontera.

La primera medición local de H2.2 observa mejoras entre 1,6x y 48,6x en las
rutas cubiertas, con 2,8x en la inversión GF(2²³³). Son resultados locales, no
garantías. Entorno, intervalos y comando están en
[`portable-optimizer.md`](portable-optimizer.md).

El orden, los gates y los entregables cerrados están en
[`phase-4-plan.md`](phase-4-plan.md) y
[`phases-3-7-roadmap.md`](phases-3-7-roadmap.md). La secuencia ejecutada está
en [`phase-4-7-plan.md`](phase-4-7-plan.md) y sus resultados en
[`phase-4-7-final-report.md`](phase-4-7-final-report.md).
