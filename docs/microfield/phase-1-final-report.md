# Informe final de la Fase 1

Fecha de cierre: 1 de agosto de 2026.

## Resultado ejecutivo

La Fase 1 de Microfield queda cerrada e integrada en `origin/main`. El resultado
es una biblioteca Rust portable para campos binarios, con tres presentaciones
congeladas, generación reproducible de especificaciones, validación matemática
independiente, API escalar sin asignaciones y un motor batch portable con
selección previa de estrategia.

El cierre funcional está contenido en `1f176ab`. El `main` integrado superó los
cinco jobs del workflow Microfield en
[`30703842091`](https://github.com/alfredoVallejoM/homomorphic-hash-rs/actions/runs/30703842091):
calidad stable, MSRV 1.89, matriz de features, artefactos y Miri.

La Fase 1 no afirma aceleración ISA. PCLMUL, VPCLMUL y PMULL quedan reservados
para la fase siguiente y no modifican los contratos cerrados aquí.

## Producto entregado

### Paquete y workspace

- El proyecto histórico se conserva como paquete raíz `homomorphic-hash-rs`.
- `microfield` es un paquete independiente dentro del mismo workspace.
- La biblioteca usa edición Rust 2024 y declara MSRV Rust 1.89.
- El paquete no se publica todavía en crates.io (`publish = false`).
- La implementación portable compila con `no_std` y sin `alloc`.
- El código de Microfield mantiene `unsafe_code = "forbid"`.

La separación permite evolucionar la biblioteca matemática sin acoplarla a
hashes, grafos, química o topología del prototipo legado.

### Arquitectura SOLID y dependencias

La biblioteca está dividida por motivos de cambio:

| Área | Responsabilidad |
|---|---|
| `field` | contratos algebraicos, encoding y metadatos públicos |
| `binary` | algoritmos genéricos de campos binarios |
| `generated` | newtypes y constantes derivados de campos certificados |
| `kernel` | ABI batch neutral, catálogos y metadatos de estrategia |
| `backend` | implementación de estrategias de ejecución |
| `engine` | selección previa, validación y fachada batch |
| `spec` | modelo, validación, planificación y generación |

Las operaciones escalares son estáticas y monomorfizadas. El motor batch
depende de `KernelSet<F>` y no de una ISA concreta. Los catálogos se asocian a
cada campo mediante `BuiltinField`, un contrato público únicamente para bounds
genéricos, oculto y sellado para impedir que consumidores registren kernels no
certificados.

No se han introducido objetos `dyn`, service locators, estado global mutable ni
heap dentro de elementos o motores. `Engine<F>` es inmutable, `Copy`, `Send` y
`Sync`.

## Especificación, certificación y generación

### Esquema v1

El esquema de manifiestos v1 acepta deliberadamente un único dominio:

- característica dos;
- extensión binaria;
- base polinómica;
- módulo mónico declarado explícitamente;
- encoding canónico little-endian.

La restricción evita aceptar hoy variantes futuras que el runtime todavía no
puede implementar correctamente.

### Pipeline

`microfield-gen` implementa un pipeline tipado y reproducible:

1. carga TOML estricta con límites de recursos;
2. normalización determinista;
3. validación estructural;
4. prueba de irreducibilidad de Rabin;
5. derivación de identidad;
6. creación de planes de producto, reducción e inversión;
7. renderizado de artefactos;
8. publicación completa mediante staging;
9. comprobación posterior de deriva.

Los casos de uso dependen de puertos; filesystem y Sage son adaptadores. La
publicación sustituye el conjunto completo y no deja staging o backups tras una
operación correcta. No se promete todavía durabilidad frente a caída del
sistema ni coordinación entre procesos concurrentes.

### Identidades

Se separaron tres conceptos:

- `FieldId`: identidad matemática y de representación;
- `ArtifactId`: identidad de una generación concreta;
- `ArtifactBundleDigest`: autenticación del conjunto publicado.

`FieldId` deriva de un JSON canónico de orden fijo. Nombres de presentación y
perfiles de build no cambian la identidad del campo, aunque sí pueden cambiar
los artefactos.

### Artefactos mantenidos

Cada campo publica `normalized.toml`, `descriptor.json`, `certificate.json`,
`generation-plan.json`, `metadata.json`, `field.rs` y `bundle.json`. La
regeneración de los tres conjuntos produce un diff vacío.

| Campo | `FieldId` |
|---|---|
| `gf2_128_v1` | `4825b6d5606e34af32722a4a6a96d04a1e21337be0fb734adb9c69f9b9d77d31` |
| `gf2_256_hh_v1` | `6b62fea68b968fd4f8c39a4f69b78f714c80858b1d0f667ec5a63d4417b43ca8` |
| `gf2_256_alt_v1` | `5c78ea2f9ea1b2d59b88bf32e38ae33be4c2f977f0232c4441f7a16e4c9bb54d` |

SageMath 10.7, ejecutado desde el entorno Conda `laboratorio_np`, generó los
tres juegos golden v2. Los vectores quedan ligados a esquema, `FieldId`,
encoding y versión del generador, y cubren las once operaciones normativas.

## Campos implementados

### Presentaciones congeladas

| Tipo público | Módulo | Tamaño/alineamiento |
|---|---|---:|
| `Gf2_128V1` | \(x^{128}+x^7+x^2+x+1\) | 16/8 bytes |
| `Gf2_256HhV1` | \(x^{256}+x^{10}+x^5+x^2+1\) | 32/8 bytes |
| `Gf2_256AltV1` | \(x^{256}+x^{16}+x^3+x+1\) | 32/8 bytes |

También se implementó `F2` como campo base.

Los tres tipos grandes son newtypes nominalmente diferentes. Sus limbs son
privados y no existen conversiones implícitas entre campos de igual
cardinalidad.

### Encoding

El contrato canónico es:

- base polinómica;
- bytes little-endian;
- bit `i` igual al coeficiente de \(x^i\);
- decodificación estricta, sin reducción silenciosa;
- reducción de polinomios arbitrarios mediante una función distinta.

No se expone `Serialize`, memoria interna, producto ancho ni representación
SIMD.

### Operaciones

Cada campo implementa:

- suma, resta y negación en característica dos;
- producto carry-less y reducción modular;
- cuadrado dedicado;
- inversión, con `None` para cero;
- potencia little-endian de longitud variable;
- multiplicación por `x`;
- reducción de bytes polinómicos arbitrarios;
- Frobenius, traza y norma;
- operadores estándar de Rust delegados en los contratos algebraicos.

La aritmética rápida se contrasta con división polinómica lenta independiente.
El cuadrado no delega en `mul(self, self)` y la inversión usa un plan generado.

### Reutilización estática

`BinaryFieldImpl` concentra el template method algebraico. Las estrategias
`Polynomial128` y `Polynomial256<TAIL>` comparten producto, reducción,
cuadrado, inversión y operaciones de extensión sin duplicar matemáticas entre
tipos públicos. El macro privado solo genera newtypes, constantes y delegación.

## API pública

La API está segregada por capacidades:

- `Field`;
- `Square`;
- `Invert`;
- `Pow`;
- `CanonicalEncoding`;
- `ExtensionField`;
- `BinaryPolynomialField`;
- `StaticField`.

Los errores públicos escalares y batch son enums tipados. No se usan strings
como categorías de error. La potencia pública es explícitamente variable-time;
la Fase 1 no ofrece una garantía general de tiempo constante.

Las features mantenidas son:

| Feature | Finalidad |
|---|---|
| `std` | integración estándar; implica `alloc` |
| `alloc` | APIs que requieren asignador |
| `portable` | motor batch portable |
| `builtin-fields` | tres tipos de campo mantenidos |
| `generator` | modelo, validación, CLI y adaptadores |
| `count-allocations` | gate de pruebas, no ruta de producción |

## Motor batch portable

`EngineBuilder<F>` aplica Builder y selección previa. Permite indicar política,
tamaño esperado o backend forzado. En esta fase solo existe el backend
portable; solicitar PCLMUL, VPCLMUL o PMULL devuelve `BackendUnavailable`.

`Engine<F>` ofrece:

- `add_into`;
- `mul_into`;
- `square_into`;
- `mul_assign`;
- `square_assign`.

Cada operación con validación comprueba longitudes antes de escribir. Un error
deja la salida intacta. Los slices vacíos son válidos y el borrow checker separa
las rutas out-of-place de las rutas in-place.

La ejecución normal realiza una validación y una llamada indirecta por lote. No
detecta CPU, asigna, empaqueta, cambia de estrategia ni crea hilos dentro del
bucle. `FixedSchedule` se rechaza porque el producto portable actual se declara
honestamente dependiente de datos.

## Calidad y verificación

### Cobertura final

- 81 tests de runtime con todos los features;
- cuatro doctests compile-fail;
- tres tests de compatibilidad con `GaloisSignature256`;
- 447 tests de la biblioteca legada;
- 26 tests de runtime habilitados bajo Miri;
- 17 tamaños batch normativos, desde 0 hasta 16 384 elementos;
- cero asignaciones observadas en las cinco operaciones batch y los tres campos.

Las suites verifican leyes algebraicas genéricas, todos los bits de base,
límites de limbs, productos densos, reducción rápida contra referencia lenta,
encoding, Fermat, Frobenius, traza, norma, vectores Sage, errores
transaccionales, canarios, features y encapsulación compile-fail.

### Matriz automatizada

El workflow ejecuta cinco jobs:

1. calidad stable: formato, tests, asignaciones, Clippy, rustdoc y legado;
2. MSRV Rust 1.89;
3. combinaciones de features y `no_std`;
4. regeneración determinista de artefactos;
5. Miri sobre el runtime portable.

Clippy y rustdoc se ejecutan con warnings denegados para Microfield.

### Rendimiento

Las operaciones escalares no contienen dispatch indirecto ni referencias al
asignador. Los tamaños públicos son exactamente 16 y 32 bytes con alineamiento
natural de 8 bytes.

Para lotes de 4096 elementos, la comparación entre bucle directo y `Engine`
observó como peor sobrecoste positivo un 1,9 % en el producto HH-256, por debajo
del gate del 3 %. El desensamblado confirma dos comparaciones de longitud y una
única llamada indirecta al kernel por operación batch. Los resultados
favorables se consideran ruido o variación de codegen, no aceleración.

## Trazabilidad de hitos

| Hito | Resultado | Evidencia principal |
|---|---|---|
| H0/H1 | scaffold, contratos y generador mínimo | artefactos certificados y reproducibles |
| H1.5 | contratos v2 y automatización | `c9671ee`, CI `30592909350` |
| H2 | vertical completo `Gf2_256HhV1` | `060fe8b`/`f3f7fc3`, CI `30622957505` |
| H3 | generalización a tres campos | `78d517f`, CI `30701163784` |
| H4 | motor batch portable | `9cbfa15`, CI de rama `30702176510` |
| Cierre | H4 integrado en `main` | `1f176ab`, CI `30703842091` |

## Criterios de terminado

La Fase 1 satisface sus criterios:

- tres campos completos y nominalmente separados;
- cuadrado especializado e inversión planificada;
- encoding estable e identidades congeladas;
- vectores externos y referencia independiente;
- batch igual a escalar;
- `no_std` sin asignador obligatorio;
- cero `unsafe` en Microfield;
- cero asignaciones escalares y batch portable;
- gates de layout, dispatch y rendimiento superados;
- documentación, ADR y CI mantenidos;
- compatibilidad semántica con el campo legado demostrada.

## Límites conscientes al cierre

- No existen todavía kernels PCLMUL, VPCLMUL o PMULL.
- No se promete tiempo constante ni `FixedSchedule` para el portable actual.
- No hay paralelismo interno, packing SIMD ni detección de CPU.
- Los artefactos no prometen publicación concurrente o durabilidad ante caída.
- La API permanece en versión `0.1.0` y el crate no se publica todavía.
- Ejemplos y benchmarks históricos del paquete legado impiden aún usar
  `cargo check --workspace --all-targets` como gate global; la matriz aislada de
  Microfield y la compatibilidad sí están verdes.

## Próxima fase recomendada

La Fase 2 revisada comienza abriendo una `BinaryFieldFactory` para que un crate
consumidor genere tipos GF(2^m) nominales, certificados y portables sin editar
Microfield. Después añade backends ISA como adaptadores internos, sin cambiar
tipos, encoding, identidad o API escalar:

1. factory estática, ABI de codegen y fixture consumidor;
2. capacidades de CPU y detección única;
3. PCLMUL para x86-64;
4. PMULL para AArch64;
5. `PackedBatch` y storage aportado;
6. VPCLMUL solo con evidencia reproducible;
7. auditoría, calibración y CI multi-ISA.

El portable cerrado en esta fase queda como especificación ejecutable y oráculo
de compatibilidad para todos esos backends.

El desarrollo completo está en [`phase-2-plan.md`](phase-2-plan.md).
