# Estado actual y siguiente plan

Fecha de revisión: 31 de julio de 2026.

## Diagnóstico ejecutivo

El scaffold H0, la Fase 0 mínima H1 y H1.5 están publicados en `origin/main`
mediante el commit `c9671ee`. Los cinco jobs de la primera matriz CI remota
terminaron correctamente en la ejecución
[`30592909350`](https://github.com/alfredoVallejoM/homomorphic-hash-rs/actions/runs/30592909350).
La arquitectura
mantiene separadas la biblioteca `no_std`, la especificación matemática, los
casos de uso y los adaptadores de I/O.

H2 está implementado localmente en la rama `agent/h2-gf2-256-hh-v1`.
`Gf2_256HhV1` es el único tipo grande público: no es un placeholder y contiene
encoding, aritmética y operaciones de extensión completas. No usa `unsafe`,
heap, dispatch dinámico ni `Engine` en el camino escalar.

| Área | Estado | Evidencia |
|---|---|---|
| Workspace e higiene | Correcto | paquete legado preservado; `target/` fuera del índice |
| API algebraica | Correcto para H2 | traits segregados, `F2` y `Gf2_256HhV1` |
| Manifiesto v1 | Correcto | parser estricto, normalización idempotente y límites de recursos |
| Identidad | Congelada | golden de `FieldId`, `ArtifactId` y `ArtifactBundleDigest` |
| Irreducibilidad | Correcto | Rabin, SymPy y ensayo independiente en grados 2–8 |
| Planes | Correcto | formas, digests, reducción y exponente de inversión comprobados |
| Emisión | Correcta a nivel de proceso | staging, reemplazo, deriva y entradas especiales probados |
| CLI | Correcta | salidas JSON/texto y códigos 0/1/2 probados |
| `no_std` | Correcto | generador opcional; runtime sin dependencias obligatorias |
| Vectores v2 | Correcto | tres goldens versionados; enum tipado, anchos, cobertura y recursos probados |
| Sage | Correcto | SageMath 10.7; tres campos regenerados con diff vacío y modelo lento independiente |
| CI H1.5 | Correcto remotamente | cinco jobs verdes en la ejecución `30592909350` |
| MSRV H2 | Correcto localmente | Rust 1.89 supera runtime y suite completa |
| Miri H2 | Correcto | 18 tests con `portable,builtin-fields` y nightly 1.96 |
| Vertical GF(2²⁵⁶) | Correcto localmente | tipo completo, Sage, referencia lenta y legado |

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
11. `Gf2_256HhV1` usa representación privada 32/8 y metadatos generados.
12. Producto carry-less, reducción word-level y cuadrado dedicado quedan
    separados por responsabilidad.
13. Inversión, potencia, `mul_by_x`, Frobenius, traza y norma están completos.
14. La compatibilidad con `GaloisSignature256` se valida byte a byte.

## Hallazgos abiertos

### Media — Alcance de «transaccional»

La publicación garantiza reemplazo completo ante errores normales del proceso,
pero no promete durabilidad frente a caída del sistema, publicación concurrente
ni atomicidad entre filesystems. Antes de usar el generador concurrentemente se
necesitará bloqueo por campo o staging con una política de coordinación.

### Operativa — H2 todavía local

H1.5 está publicado y recuperable. Los cambios H2 permanecen sin commit en su
rama de trabajo y necesitarán su propia revisión y ejecución CI antes de
integrarse.

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

La suite de Microfield contiene 72 tests y la compatibilidad legada añade 3:

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
  independiente para los tres campos;
- 128 productos, cuadrados y desplazamientos contra reducción bit a bit;
- 48 tríos deterministas para leyes de campo;
- inversión de Fermat, Frobenius, traza, norma y polinomios de hasta 97 bytes;
- las 11 operaciones Sage ejecutadas contra la API pública;
- 64 comparaciones de encoding, suma, producto y fase con el tipo legado, más
  8 inversiones.

## Rendimiento estructural H2

El release `no_std` no contiene símbolos del asignador. La rutina algebraica de
inversión no contiene indirect calls; las únicas indirecciones observadas en
el objeto corresponden al protocolo de `Formatter`, fuera del hot path.

El harness Criterion registra una línea base local reproducible por operación.
En el i7-13700HX usado para esta revisión: multiplicación 461 ns, cuadrado
12,16 ns, `mul_by_x` 1,57 ns, reducción de 64 bytes 503 ns e inversión
123,22 µs. Son datos de comparación local, no garantías portables.

## Orden recomendado

### H2 — Vertical `Gf2_256HhV1`

Todos los puntos del vertical están implementados localmente. Antes de H3:

1. revisar el diff H2;
2. crear un commit específico;
3. subir la rama y observar la nueva matriz CI.

Salida alcanzada: un único campo grande completo y portable, todavía sin batch
ni ISA.

### H3 — Generalización

Generar `Gf2_128V1` y `Gf2_256AltV1` usando el mismo IR y algoritmos, añadir
compile-fail de mezcla de campos y demostrar que no existe lógica matemática
duplicada.

### H4 — Batch portable

Introducir `KernelSet`, catálogo sellado, `EngineBuilder`, validación previa de
slices, tests de canarios/asignaciones y benchmark del único dispatch por lote.
