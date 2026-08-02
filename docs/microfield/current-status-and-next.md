# Estado actual y siguiente plan

Fecha de revisión: 2 de agosto de 2026.

## Diagnóstico ejecutivo

Actualización de Fase 3: los algoritmos derivados están implementados y
cerrados localmente. Existen inversión batch tolerante a cero, máscara compacta,
workspace tipado, scans, las dos orientaciones de Horner, `mul_add_into` y
potencias fijas. El planner usa schema 2/IR v4 y verifica simbólicamente la
cadena Itoh–Tsujii exacta antes de calcular artefactos. La ruta prestada pasa
el gate de cero asignaciones y no amplía `unsafe`.

La planificación Fases 3–7 se corrigió en
[`phases-3-7-roadmap.md`](phases-3-7-roadmap.md). En particular, Fase 6
comenzará con la rehabilitación completa del legado sobre campos `microfield`
y contendrá un track exacto de canonización de grafos; las firmas algebraicas
solo podrán acelerar refinamiento, nunca decidir isomorfismo por sí solas.

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
| Fase 3 | Cerrada localmente | algoritmos derivados, IR v4, Miri/ASan y benchmark |
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
| Frontera `unsafe` | Confinada y autenticada | `deny` global, cuatro hashes revisados y test estructural |
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
y, tras H2.7, permite exactamente cuatro excepciones `unsafe`: tres adapters
ISA y el único módulo de storage alineado.

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

El paquete legado conserva 447 tests correctos, pero sus benchmarks y ejemplos
históricos impiden todavía `cargo check --workspace --all-targets`; también hay
formato legado pendiente. La matriz aislada de Microfield y compatibilidad no
está afectada.

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

### Siguiente fase

Fase 3 está cerrada. El siguiente corte es F4.0: congelar campos primos,
certificados y representación antes de implementar `Fp251V1`, Goldilocks y un
primo multi-limb. Los algoritmos de Fase 3 se reutilizarán sin duplicación y
servirán como test arquitectónico de que no estaban acoplados a GF(2^m).

La primera medición local de H2.2 observa mejoras entre 1,6x y 48,6x en las
rutas cubiertas, con 2,8x en la inversión GF(2²³³). Son resultados locales, no
garantías. Entorno, intervalos y comando están en
[`portable-optimizer.md`](portable-optimizer.md).

El orden, los gates y los entregables están en
[`phase-3-plan.md`](phase-3-plan.md) y
[`phases-3-7-roadmap.md`](phases-3-7-roadmap.md).
