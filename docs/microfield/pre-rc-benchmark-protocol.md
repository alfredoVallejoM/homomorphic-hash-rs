# Protocolo pre-RC de benchmarks publicables

Fecha: 9 de agosto de 2026.

Estado: protocolo v1 congelado para implementación. Este documento sustituye
la idea anterior de integrar primero la RC y medir después. La rama actual es
un candidato técnico interno, pero no se integrará ni etiquetará como RC hasta
que las fases B.1–B.3 definidas aquí produzcan evidencia reproducible.

Las firmas o «hashes homomórficos» de este proyecto son resúmenes algebraicos
**no criptográficos**. Los benchmarks no medirán ni sugerirán resistencia a
colisiones adversariales, autenticación, seguridad o tiempo constante.

## 1. Preguntas que debe responder la evidencia

La campaña no se limita a preguntar qué función tarda menos. Debe separar y
responder, con inputs, escalas y unidades publicadas:

1. cuánto cuestan las operaciones de campo y qué ganan batch, packed e ISA;
2. cuánto cuesta construir, combinar, actualizar, serializar y validar cada
   familia de firma;
3. en qué escalas una actualización incremental supera una recomposición
   exacta y dónde debe activarse el fallback;
4. qué memoria, asignaciones, bytes wire y bytes persistentes paga cada ruta;
5. cómo escala el resultado y qué parte corresponde a setup, cálculo,
   generación de delta, aplicación, serialización, persistencia o I/O;
6. qué resultados se repiten en otro proceso, máquina y arquitectura;
7. qué claims siguen siendo inconclusos por ruido, falta de precisión o falta
   de un baseline semánticamente equivalente.

RC.8 seguirá siendo el gate rápido de capacidad y regresión. Esta campaña es
otro producto: conserva observaciones crudas y está diseñada para auditoría y
presentación externa.

## 2. Fundamento metodológico

El protocolo adopta estas consecuencias prácticas de fuentes primarias y
normas de publicación:

- Kalibera y Jones distinguen niveles de variación y recomiendan asignar la
  repetición al nivel que realmente domina la incertidumbre, además de
  presentar tamaño de efecto con intervalo de confianza
  ([DOI 10.1145/2464157.2464160](https://doi.org/10.1145/2464157.2464160)).
- Georges, Buytaert y Eeckhout muestran que una evaluación de rendimiento
  debe cubrir ejecuciones independientes y análisis estadístico explícito, no
  solo iteraciones dentro del mismo proceso
  ([DOI 10.1145/1297105.1297033](https://doi.org/10.1145/1297105.1297033)).
- El bootstrap no paramétrico procede del trabajo de Efron
  ([DOI 10.1214/aos/1176344552](https://doi.org/10.1214/aos/1176344552));
  aquí se aplicará respetando la jerarquía proceso/observación para no crear
  independencia artificial.
- Criterion.rs usa warmup, muestreo y bootstrap para estimar distribuciones e
  intervalos. Se mantiene como referencia para microbenchmarks, pero el
  laboratorio propio es necesario para procesos independientes, artefactos
  end-to-end y comparaciones pareadas
  ([análisis oficial](https://bheisler.github.io/criterion.rs/book/analysis.html)).
- `std::hint::black_box` solo es una barrera de optimización best-effort. Los
  inputs y outputs deberán cruzarla y cada celda tendrá además un checksum
  semántico verificable
  ([documentación de Rust](https://doc.rust-lang.org/stable/std/hint/fn.black_box.html)).
- Las reglas SPEC exigen revelar aquello que afecta al rendimiento: hardware,
  software, flags, tuning, energía/temperatura cuando corresponda y condiciones
  de ejecución
  ([SPEC CPU 2017, run and reporting rules](https://www.spec.org/cpu2017/Docs/runrules.html)).
- La evaluación de artefactos de ACM separa repeatability, reproducibility y
  replicability. La primera meta es que código, datos crudos y transformación
  reproduzcan el resultado en las condiciones declaradas
  ([ACM Artifact Review and Badging](https://www.acm.org/publications/policies/artifact-review-and-badging-current)).

Estas fuentes orientan el diseño; no se afirmará conformidad SPEC ni la
obtención de una insignia ACM.

## 3. Unidad experimental y estadística

### 3.1 Jerarquía de observaciones

Cada celda se ejecutará en procesos nuevos. Un proceso contiene:

1. construcción determinista del input desde `seed`;
2. verificación semántica previa;
3. calibración del número de operaciones por observación;
4. warmup fuera de las muestras;
5. observaciones crudas cronometradas;
6. checksum final y metadatos de ejecución.

El proceso es la réplica independiente primaria. Las observaciones de un mismo
proceso estiman la distribución interna, pero no incrementan falsamente el
número de procesos.

### 3.2 Repetición adaptativa

- `smoke`: 2 procesos, 3 warmups y 5 observaciones; solo valida el harness.
- `pilot`: 10 procesos, 5 warmups y 15 observaciones; estima varianza y coste.
- `publication`: mínimo 30 y máximo 100 procesos por celda, 10 warmups y 50
  observaciones por proceso.
- después de 30 procesos se comprueba la precisión cada 5 procesos;
- una celda puede parar cuando el intervalo bootstrap del 95 % de la mediana
  tenga semianchura relativa <= 5 %;
- si alcanza 100 procesos sin esa precisión, se publica como `Inconclusive` y
  no se deriva un claim cuantitativo fuerte.

Los límites son defaults versionados. El piloto puede justificar aumentarlos,
nunca reducirlos silenciosamente.

### 3.3 Operaciones rápidas y calibración

Una multiplicación de campo puede estar cerca de la resolución/overhead del
reloj. Antes de medir se duplica el batch interno hasta alcanzar al menos 1 ms
por observación o el límite configurado. Se informa tiempo por operación y
también el batch usado; no se resta un supuesto «overhead del reloj».

### 3.4 Orden y emparejamiento

- el orden de celdas se baraja mediante un PRNG determinista y se registra;
- variantes de una comparación usan el mismo input y la misma réplica;
- el orden A/B se alterna de forma determinista para evitar favorecer siempre
  a la primera variante;
- directo, fachada comprobada, incremental y rebuild se analizan como
  contrastes pareados cuando comparten semántica;
- un baseline con semántica distinta se etiqueta como contexto, no como
  ganador/perdedor.

### 3.5 Estadísticos

Por celda y contraste se publicarán:

- todas las duraciones crudas y su pertenencia a proceso/observación;
- mediana, p95, p99, MAD y throughput;
- intervalo bootstrap percentil del 95 % de la mediana;
- intervalo bootstrap jerárquico del 95 % para p95/p99;
- ratio pareado incremental/rebuild y su intervalo del 95 %;
- pendiente de la curva log-log con intervalo, solo cuando haya al menos cinco
  escalas válidas;
- número de procesos, observaciones, outliers señalados y muestras fallidas.

No se borrarán outliers del resultado principal. Una vista exploratoria puede
marcarlos usando MAD, conservando siempre el dato original y la razón.

## 4. Control y revelación del entorno

El worker registrará, cuando el sistema lo exponga:

- commit, estado limpio, versión del manifest y digest del binario;
- `rustc`, `cargo`, LLVM, target, profile, features y `RUSTFLAGS`;
- OS, kernel, arquitectura, hostname anonimizable y contenedor/VM;
- modelo y vendor CPU, sockets, cores, threads, cache y NUMA;
- microcode, flags ISA, CPU affinity y CPUs permitidas;
- governor, driver de frecuencia, frecuencia mínima/máxima/actual y turbo;
- memoria total/disponible, page size y swap;
- temperatura antes/después si existe sensor legible;
- filesystem y dispositivo para celdas con persistencia;
- fecha UTC, zona horaria, locale y procesos concurrentes configurados.

El launcher no modificará governor, turbo, IRQ, afinidad ni límites del host
sin una acción explícita del operador. En su lugar valida precondiciones y
clasifica la ejecución:

- `controlled`: afinidad fijada, host dedicado y campos críticos conocidos;
- `informative`: ejecución válida, pero uno o más controles no están fijados;
- `smoke`: CI compartida; no publicable.

Solo `controlled` puede sustentar las cifras principales de una presentación.

## 5. Matriz completa de la propuesta

### K — núcleo de campos finitos

| Familia | Operaciones | Variantes | Escalas |
|---|---|---|---|
| binarios mantenidos | add, mul, square, invert, encode/decode | GF(2^128), GF(2^256)-HH y alternativo; portable/detected | batch calibrado; 1–65.536 elementos |
| primos mantenidos | add, sub, mul, square, invert, encode/decode | Fp251, Goldilocks y perfiles grandes disponibles | batch calibrado; 1–65.536 elementos |
| batch/packed | pack, kernel, unpack y pipeline | scalar, portable batch, packed, ISA seleccionada | 1, 8, 64, 512, 4.096, 65.536 lanes |
| runtime/generado | parse, validate, build, select y operación | estático, runtime y artefacto generado equivalente | manifests y batches representativos |
| algoritmos derivados | powers, Horner, scan, mask, batch invert | workspace nuevo/reutilizado | 1–65.536 elementos |

El ID `gf2_256_hh_v1` conserva «hh» como identidad histórica del campo; no
convierte la operación o la firma en criptográfica.

### S — firmas algebraicas homomórficas

| Familia | Rutas medidas | Comparación exacta |
|---|---|---|
| aditiva | build, push, merge directo/fachada, delta apply/rollback, wire | rebuild desde items |
| secuencia | build, push, concatenate, append/trim, restore, wire | rebuild preservando orden |
| bidireccional | build, concatenate, reverse/restore, wire | rebuild en ambos sentidos |
| multiconjunto | build, insert/remove, merge, delta, restore, wire | rebuild con multiplicidad exacta |
| multievaluación K | build, merge/concatenate, update, wire | misma familia K y rebuild |
| runtime | build/update/wire con contexto validado | tipo estático equivalente cuando exista |

Escalas de items: 0, 1, 8, 64, 512, 4.096, 65.536 y 1.048.576 cuando la
memoria del perfil lo permita. Payloads: 1, 16, 128 y 4.096 bytes. Se separará
generación/encoding del álgebra.

### A — estructuras y verticales de aplicación

| Área | Rutas | Eje de escala y baseline |
|---|---|---|
| delta/journal | generate, validate, apply, rollback, replay | número de operaciones; rebuild exacto |
| summary tree | local edit, append, boundary change, serialize | bytes editados/tamaño total; full rebuild |
| DB en memoria | transaction generate/apply, policy fallback, log/replay, reconcile | mutaciones/filas/particiones; table rebuild |
| grafos | build/prepare, fast filter, exact microcanon, incremental, DAG reuse | V/E, rondas, budget y degeneración; recompute |
| herramientas | manifest parse/validate, planner, generate, verify, filesystem publish | tamaño/field; proceso frío y caliente |

Los benchmarks de DB con WAL/fsync y concurrencia pertenecen a la fase
posterior B.4. En B.1–B.3 se mide de forma exhaustiva el motor en memoria y se
preserva explícitamente esta frontera.

## 6. Baselines y claims permitidos

Baselines internos obligatorios:

- backend portable frente a seleccionado, manteniendo campo e input;
- scalar frente a batch/packed, manteniendo operación;
- operación de campo directa frente a fachada con checks;
- actualización incremental frente a rebuild exacto;
- workspace nuevo frente a reutilizado;
- proceso frío frente a caliente para herramientas.

SHA-256 puede aparecer solo como referencia de coste de un digest completo.
No tiene la misma semántica ni el mismo nivel de seguridad y no será usado
para afirmar superioridad. Tampoco se comparará con crates externas hasta
definir operación, representación, optimizaciones y semántica equivalentes.

Claims que sí puede sostener la campaña:

- throughput/latencia observados bajo el entorno declarado;
- speedup entre dos rutas semánticamente equivalentes;
- punto de equilibrio observado e intervalo para incremental/rebuild;
- tendencia de escala dentro del rango medido;
- coste de checks, wire, asignación o persistencia separado por etapa.

## 7. Artefactos reproducibles

La implementación B.2 generará, sin sobrescribir campañas anteriores:

```text
validation/benchmarks/
  protocol-v1.json
  manifests/{smoke,pilot,publication}-v1.json
  schema/*.schema.json
  runs/<campaign-id>/
    manifest.json
    environment.json
    execution-order.json
    raw/*.jsonl
    aggregate.json
    aggregate.csv
    comparisons.csv
    checksums.txt
    report.md
```

Los `runs/` publicables se versionarán solo cuando identifiquen commit y host,
pasen el validador y no contengan datos privados. CI conservará los smoke como
artifacts efímeros.

## 8. Gates antes de la RC

### B.1 — investigación y protocolo

- protocolo, matriz, fronteras de claims y criterios estadísticos revisados;
- expectativas documentales corregidas: benchmark antes de integrar RC;
- ninguna cifra de RC.8 presentada como resultado publicable.

### B.2 — harness y análisis

- launcher y worker multiproceso con orden reproducible;
- manifests validados, raw JSONL, agregación y bootstrap determinista;
- calibración de operaciones rápidas y checksums semánticos;
- tests unitarios de percentiles, MAD, bootstrap, pairing y schemas;
- smoke completo en CI para todas las familias, con escalas reducidas.

### B.3 — ejecución y curvas

- piloto completo local para descubrir varianza y coste;
- campaña `publication` controlada o, si el host no cumple, campaña
  `informative` claramente marcada;
- curvas de cinco o más escalas para directo/incremental/rebuild;
- informe regenerable desde raw, sin edición manual de cifras;
- resultados inconclusos identificados, nunca ocultos.

Solo después de B.1–B.3 se reevalúa si existe una RC. La integración, PR, merge
y tag quedan aplazados; no forman parte de estas tres fases.

## 9. Criterio de cierre por fase

Cada fase termina únicamente con documentación/código/tests coherentes,
validación local, commit específico, push inmediato y CI remota verde. El
informe final incluirá el SHA exacto y distinguirá ejecución local,
informativa y controlada.
