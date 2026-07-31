# Estado actual y siguiente plan

Fecha de revisión: 31 de julio de 2026.

## Diagnóstico ejecutivo

H0, H1, H1.5 y H2 están integrados en `origin/main`. H2 entró mediante el
commit `f3f7fc3`; sus cinco jobs y los cinco jobs posteriores de `main`
terminaron correctamente en las ejecuciones
[`30622165087`](https://github.com/alfredoVallejoM/homomorphic-hash-rs/actions/runs/30622165087)
y
[`30622957505`](https://github.com/alfredoVallejoM/homomorphic-hash-rs/actions/runs/30622957505).

H3 está implementado y validado localmente en `agent/h3-generalization`.
`Gf2_128V1`, `Gf2_256HhV1` y `Gf2_256AltV1` son tipos públicos completos que
comparten estrategias estáticas y algoritmos binarios. Los elementos conservan
representación privada, identidad nominal y layout natural; el camino escalar
no usa `unsafe`, heap, dispatch dinámico ni `Engine`.

| Área | Estado | Evidencia |
|---|---|---|
| Workspace e higiene | Correcto | paquete legado preservado; `target/` fuera del índice |
| API algebraica | Correcto para H3 | traits segregados, `F2` y tres campos binarios públicos |
| Generalización | Correcta localmente | `BinaryFieldImpl`, estrategias 128/256 y algoritmos compartidos |
| Manifiesto v1 | Correcto | parser estricto, normalización idempotente y límites de recursos |
| Identidad | Congelada | golden de `FieldId`, `ArtifactId` y `ArtifactBundleDigest` |
| Irreducibilidad | Correcto | Rabin, SymPy y ensayo independiente en grados 2–8 |
| Planes | Correcto | formas, digests, reducción y exponente de inversión comprobados |
| Emisión | Correcta a nivel de proceso | staging, reemplazo, deriva y entradas especiales probados |
| CLI | Correcta | salidas JSON/texto y códigos 0/1/2 probados |
| `no_std` | Correcto | generador opcional; runtime sin dependencias obligatorias |
| Vectores v2 | Correcto | tres goldens; enum tipado, anchos, cobertura y recursos probados |
| Sage | Correcto | 33 ejecuciones: 11 operaciones sobre cada API pública |
| CI H2 | Correcto remotamente | rama y `main` verdes en `30622165087` y `30622957505` |
| MSRV H3 | Correcto localmente | Rust 1.89 supera runtime y doctests |
| Miri H3 | Correcto localmente | 23 tests de runtime y 2 doctests sin UB |
| Ensamblado H3 | Correcto localmente | sin asignador ni llamadas indirectas algebraicas |

## Decisiones materializadas en H3

1. `BinaryFieldImpl` segrega el value object público de la estrategia
   matemática interna.
2. `Polynomial128<TAIL>` y `Polynomial256<TAIL>` son descriptores estáticos;
   no guardan estado ni exigen dispatch virtual.
3. Producto ancho, reducción, cuadrado, inversión, Frobenius y traza se
   implementan una vez y se monomorfizan para cada grado y tail.
4. Un macro privado emite solo newtypes, metadatos, operadores y delegación;
   no duplica la lógica algebraica.
5. `Gf2_256HhV1` y `Gf2_256AltV1` no se pueden mezclar ni convertir
   implícitamente, aunque ambos tengan 256 bits.
6. Los limbs y productos anchos permanecen privados.
7. Las suites genéricas ejercitan las mismas leyes sobre los tres campos.
8. El workflow ejecuta doctests compile-fail para congelar las fronteras de
   tipo y representación.

La motivación y las alternativas se registran en
[`ADR 0008`](adr/0008-static-field-generalization.md).

## Hallazgos abiertos

### Media — Alcance de «transaccional»

La publicación garantiza reemplazo completo ante errores normales del proceso,
pero no promete durabilidad frente a caída del sistema, publicación concurrente
ni atomicidad entre filesystems. Antes de usar el generador concurrentemente se
necesitará bloqueo por campo o staging con una política de coordinación.

### Operativa — integración de H3

`agent/h3-generalization` es la unidad de entrega de la implementación y sus
pruebas. No debe integrarse hasta revisar su commit como unidad y exigir la
misma matriz CI remota que protegió H2.

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

Estos fallos no bloquean H3 porque la matriz de Microfield y la compatibilidad
legada se ejecutan de forma aislada, pero impiden afirmar que todos los targets
históricos del workspace están verdes.

## Cobertura H3

La suite de Microfield contiene 77 tests de runtime, dos doctests compile-fail
y la compatibilidad legada añade 3:

- leyes exhaustivas, encoding, potencia, layout y formato de `F2`;
- contratos del manifiesto, normalización, identidad, Rabin, planes, CLI y
  publicación transaccional;
- esquema de vectores v2, límites de recursos y regeneración determinista;
- todos los bits de la base canónica para los tres campos públicos;
- productos, cuadrados y `mul_by_x` contra un modelo polinómico independiente;
- leyes de campo, inversión, potencia, Frobenius, traza y norma genéricas;
- reducción de entradas polinómicas de longitudes arbitrarias;
- las 11 operaciones Sage ejecutadas sobre cada uno de los tres tipos;
- distinción matemática y nominal de los dos campos de 256 bits;
- compile-fail al mezclar campos o intentar construir limbs privados;
- 64 comparaciones de encoding, suma, producto y fase con el tipo legado, más
  8 inversiones.

## Rendimiento estructural H3

El release `no_std` no contiene símbolos del asignador. Las rutinas de
inversión solo realizan llamadas directas a instancias monomorfizadas de
producto/cuadrado; no aparecen llamadas indirectas algebraicas.

Medición orientativa del 31 de julio de 2026, Rust 1.97.1, release, Linux
x86-64 e Intel Core i7-13700HX:

| Campo | Multiplicación | Cuadrado | `mul_by_x` | Reducción | Inversión |
|---|---:|---:|---:|---:|---:|
| `Gf2_128V1` | 111,17 ns | 6,97 ns | 0,85 ns | 219,30 ns | 16,36 µs |
| `Gf2_256HhV1` | 460,10 ns | 11,92 ns | 1,59 ns | 518,14 ns | 118,38 µs |
| `Gf2_256AltV1` | 460,04 ns | 11,83 ns | 1,57 ns | 519,89 ns | 116,57 µs |

La multiplicación HH permanece en la misma banda que la línea base H2 de
461 ns. El cambio de compilador impide interpretar diferencias pequeñas como
una mejora o regresión estricta.

## Orden recomendado

### Cierre de H3

1. revisar el commit atómico de `agent/h3-generalization`;
2. exigir CI verde en la rama;
3. integrar por fast-forward en `main` y repetir CI.

Salida: tres presentaciones nominalmente distintas que reutilizan un único
núcleo algebraico portable.

### H4 — Batch portable

Introducir `KernelSet`, catálogo sellado, `EngineBuilder`, validación previa de
slices, operaciones out-of-place/in-place, tests de canarios y asignaciones, y
benchmark del único dispatch por lote.
