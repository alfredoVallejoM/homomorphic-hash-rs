# Estado actual y siguiente plan

Fecha de revisión: 31 de julio de 2026.

## Diagnóstico ejecutivo

El scaffold H0, la Fase 0 mínima H1 y H1.5 están implementados y validados
localmente. La arquitectura
mantiene separadas la biblioteca `no_std`, la especificación matemática, los
casos de uso y los adaptadores de I/O. No existen tipos públicos ficticios para
GF(2¹²⁸) o GF(2²⁵⁶), ni `unsafe`, dispatch dinámico o asignaciones incorporadas
al elemento de campo.

La base matemática ya es adecuada para comenzar el vertical portable. SageMath
10.7 generó los tres juegos golden v2, una implementación polinómica lenta los
verifica dentro de la suite y la regeneración es idéntica byte a byte. Quedan
dos tareas operativas antes de fijar la línea base: observar la matriz CI remota
y crear un commit revisable.

| Área | Estado | Evidencia |
|---|---|---|
| Workspace e higiene | Correcto | paquete legado preservado; `target/` fuera del índice |
| API algebraica | Correcto para el alcance | traits segregados y `F2` exhaustivamente probado |
| Manifiesto v1 | Correcto | parser estricto, normalización idempotente y límites de recursos |
| Identidad | Congelada | golden de `FieldId`, `ArtifactId` y `ArtifactBundleDigest` |
| Irreducibilidad | Correcto | Rabin, SymPy y ensayo independiente en grados 2–8 |
| Planes | Correcto | formas, digests, reducción y exponente de inversión comprobados |
| Emisión | Correcta a nivel de proceso | staging, reemplazo, deriva y entradas especiales probados |
| CLI | Correcta | salidas JSON/texto y códigos 0/1/2 probados |
| `no_std` | Correcto | generador opcional; runtime sin dependencias obligatorias |
| Vectores v2 | Correcto | tres goldens versionados; enum tipado, anchos, cobertura y recursos probados |
| Sage | Correcto | SageMath 10.7; tres campos regenerados con diff vacío y modelo lento independiente |
| CI/MSRV | Correcto localmente, pendiente de ejecución remota | Rust 1.89 supera los 62 tests y el runtime `no_std`; workflow completo definido |
| Miri | Correcto para el runtime actual | 9 tests `--no-default-features` superados con nightly 1.96 |
| Vertical GF(2²⁵⁶) | No iniciado | no se exporta ningún tipo grande |

## Correcciones introducidas durante esta revisión

1. El manifiesto queda limitado a 64 KiB antes de parsear.
2. El grado v1 tiene un techo absoluto de 4096 que ninguna configuración puede
   elevar; el builder solo puede imponer límites menores.
3. La cantidad de términos del módulo se acota antes de construir estructuras
   auxiliares.
4. Una publicación existente que sea fichero o symlink se rechaza sin
   reemplazarla.
5. `check` detecta ficheros extra, directorios vacíos y rechaza symlinks o
   entradas especiales.
6. Se añadieron accessors verificables para dimensiones de producto/reducción
   y para el límite efectivo del validador.
7. Los vectores usan un esquema v2 tipado con cobertura normativa completa.
8. Cada publicación incluye `bundle.json` y `ArtifactBundleDigest`.
9. `karatsuba` se rechaza mientras no exista una implementación medida.
10. Se añadió una matriz CI reproducible.

## Hallazgos abiertos

### Media — Alcance de «transaccional»

La publicación garantiza reemplazo completo ante errores normales del proceso,
pero no promete durabilidad frente a caída del sistema, publicación concurrente
ni atomicidad entre filesystems. Antes de usar el generador concurrentemente se
necesitará bloqueo por campo o staging con una política de coordinación.

### Operativa — Cambios todavía sin commit

Los ficheros de Microfield continúan sin seguimiento y la retirada de 5443
entradas históricas de `target/` está staged. Debe hacerse un commit deliberado
antes de continuar para que la base sea recuperable y revisable.

### Fuera de alcance

El paquete legado conserva 447 tests de biblioteca correctos, pero
`cargo check --workspace --all-targets` todavía falla fuera de Microfield:

- `exp_a_merkle_asymptotics` no declara `rs_merkle` y conserva tres usos
  genéricos obsoletos de `MultisetAggregator`;
- `exp_b_causal_rollback` conserva tres usos genéricos obsoletos de
  `SequenceAggregator`;
- `chemistry_paper` llama al método inexistente `ureq::Response::body_mut`;
- `cargo fmt --all --check` detecta formato histórico pendiente en el paquete
  legado.

Estos fallos no deben bloquear H2 salvo que se decida incluir todo el legado en
la matriz global, pero sí impiden afirmar que el workspace completo está verde.

## Nueva cobertura

La suite de Microfield contiene 62 tests:

- leyes exhaustivas, encoding, potencia, layout y formato de `F2`;
- claves desconocidas en todas las capas del TOML;
- valores fijos no soportados y formas polinómicas inválidas;
- límites de tamaño en parser y loader, y grado no anulable;
- normalización idempotente y build canónico;
- 247 candidatos mónicos de grados 2–8 comparados con división por ensayo;
- identidades, certificados, planes y digests golden;
- reducción por plan contra división polinómica;
- reconstrucción exacta del exponente `2^m-2`;
- consistencia y digest independiente de los siete ficheros de cada artefacto;
- determinismo entre generaciones independientes;
- significado preciso del cambio de nombre;
- deriva, ficheros extra, directorios vacíos, symlinks y rollback conservador;
- contratos CLI de éxito, error y deriva;
- esquema v2 completo, cobertura obligatoria y operaciones desconocidas;
- anchos canónicos y anchos no alineados a byte mediante GF(2⁵);
- producto ancho, bits de padding, exponentes e inversión cero/no-cero;
- límites de 8 MiB, 4096 casos y 4096 bytes de exponente;
- importación, publicación y regeneración estable de los JSON golden;
- contraste de todas las operaciones Sage con un modelo polinómico lento e
  independiente para los tres campos.

## Orden recomendado

### H1.5 — Estabilización previa al vertical

1. Ejecutar en GitHub la matriz ya definida; stable y MSRV 1.89 ya están
   confirmados localmente junto con
   `no_std`, Clippy, rustdoc, artefactos y biblioteca legada.
2. Repetir Miri cuando H2 introduzca la aritmética portable; el runtime actual
   ya supera sus 9 tests aplicables con Miri nightly 1.96.
3. Crear un commit base revisable después de comprobar el diff de `target/`.

### H2 — Vertical `Gf2_256HhV1`

1. Introducir `#[repr(transparent)] Gf2_256HhV1([u64; 4])` con limbs privados.
2. Implementar encoding canónico y una referencia polinómica lenta.
3. Implementar `clmul64` portable y producto escolar ancho.
4. Ejecutar la reducción generada y contrastarla con división lenta para
   vectores dirigidos y property tests reproducibles.
5. Añadir cuadrado dedicado, `mul_by_x`, potencia e inversión por plan.
6. Implementar Frobenius, traza y norma.
7. Ejecutar leyes genéricas, vectores Sage y compatibilidad byte a byte con
   `GaloisSignature256`.
8. Verificar tamaño 32, alineamiento 8, cero asignaciones y desensamblado
   escalar sin llamadas indirectas.

Salida: un único campo grande completo y portable, todavía sin batch ni ISA.

### H3 — Generalización

Generar `Gf2_128V1` y `Gf2_256AltV1` usando el mismo IR y algoritmos, añadir
compile-fail de mezcla de campos y demostrar que no existe lógica matemática
duplicada.

### H4 — Batch portable

Introducir `KernelSet`, catálogo sellado, `EngineBuilder`, validación previa de
slices, tests de canarios/asignaciones y benchmark del único dispatch por lote.
