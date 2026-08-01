# Auditoría inicial de Microfield

## Estado observado

- El repositorio era un único paquete `homomorphic-hash-rs`.
- La biblioteca legada mantiene 447 pruebas unitarias correctas.
- La compilación de todos los targets legados falla por dependencias y APIs
  desactualizadas en benchmarks y ejemplos; no es un fallo de Microfield.
- `target/` estaba versionado con 5.443 entradas y aproximadamente 1,8 GB.
- `planificacion.md` estaba doblemente codificado y referenciaba imágenes
  inexistentes.

## Defectos de la planificación original

| Severidad | Hallazgo | Resolución |
|---|---|---|
| Bloqueante | El repositorio existente y el paquete `microfield` tenían nombres y responsabilidades incompatibles. | Workspace con migración gradual. |
| Bloqueante | La Fase 1 dependía de polinomios todavía no congelados. | Tres manifiestos normativos con exponentes explícitos. |
| Alta | `engine` y `backend` formaban una dependencia conceptual circular. | Módulo neutral `kernel` y catálogo entregado por el campo. |
| Alta | `unsafe fn` en el ABI batch contradecía la Fase 1 sin `unsafe`. | Estrategias seguras sobre slices; `unsafe` local solo en Fase 2. |
| Alta | `Field` acumulaba capacidades no requeridas por todos los consumidores. | Traits `Square`, `Invert` y `Pow` segregados. |
| Alta | El tipo legado alineado a 32 bytes no podía sustituirse sin ruptura. | Compatibilidad semántica por encoding, no alias de layout. |
| Media | La fórmula de producto escolar no distinguía mitades de `u128`. | El contrato técnico fija acumulación baja/alta en limbs consecutivos. |
| Media | Imágenes y documento de procedencia no estaban presentes. | Diagramas Mermaid y ADR mantenidos dentro del repositorio. |

## Línea base

El scaffold inicial no afirmaba que GF(2¹²⁸) o GF(2²⁵⁶) estuvieran
implementados. El hito H1 posterior certifica sus tres polinomios y genera
datos y planes, pero mantiene la misma frontera honesta: el único campo
ejecutable nuevo continúa siendo `F2` hasta completar el vertical portable.

## Cierre de H1

- Parser TOML v1 estricto y normalización determinista.
- `FieldId` y `ArtifactId` separados por semántica y representación.
- Rabin independiente con certificados repetibles para los tres módulos.
- Planes de producto, reducción descendente e inversión serializables.
- CLI funcional y fachada de casos de uso independiente de I/O.
- Puertos de publicación/oráculo, filesystem transaccional y adaptador Sage.
- Tests golden, reducibles, identidad, deriva y regeneración byte a byte.

La revisión del 31 de julio amplió inicialmente la suite a 49 tests y añadió
límites duros de recursos, ensayo independiente para los 247 candidatos
mónicos de grados 2–8, contratos de CLI, planes de inversión, consistencia
cruzada de artefactos y casos adversariales de filesystem. H1.5 cerró con 62
tests y fue publicado en `c9671ee`; los cinco jobs del workflow remoto
`30592909350` terminaron correctamente.

H2 elevó la suite de Microfield a 72 tests, añadió 3 contratos contra el tipo
legado y ejecutó 18 tests bajo Miri con el campo grande habilitado. La matriz
local pasó con Rust estable 1.93.1, Clippy y rustdoc con warnings denegados; la
compatibilidad MSRV se mantuvo en Rust 1.89. H2 está integrado en `main` en
`f3f7fc3` y los workflows `30622165087` y `30622957505` terminaron verdes.
Los hallazgos y el siguiente orden de trabajo están en
[`current-status-and-next.md`](current-status-and-next.md).

SageMath 10.7, ejecutado desde el entorno Conda `laboratorio_np`, generó los
tres juegos de vectores v2. Los ficheros se importan mediante el contrato
tipado, coinciden byte a byte al regenerarlos y todas sus operaciones se
contrastan además con un modelo polinómico lento escrito en Rust.

Como contraste adicional disponible localmente, SymPy confirmó de forma
independiente que los tres polinomios congelados son irreducibles. Esta
segunda comprobación no sustituye los vectores operacionales de Sage.

H1.5 amplía la suite a 62 tests, congela el esquema tipado de vectores v2,
separa `ArtifactBundleDigest` de `ArtifactId`, añade `bundle.json`, rechaza
estrategias sin implementación y define la matriz CI. H2 implementa
`Gf2_256HhV1` completo, contrasta 128 productos con reducción lenta, consume
los vectores Sage y demuestra compatibilidad semántica con
`GaloisSignature256`.

H3 amplía la API a los tres campos congelados mediante un contrato interno y
dos estrategias estáticas de ancho. La matemática se comparte y monomorfiza;
el macro privado se limita a emitir newtypes y delegación. La suite alcanza 77
tests de runtime y dos doctests compile-fail, y ejecuta las 11 operaciones Sage
sobre cada tipo público. Miri supera los 23 tests de runtime habilitados y los
dos doctests sin detectar comportamiento indefinido.

H3 fue integrado en `main` mediante `78d517f`; las ejecuciones de rama y main
`30624475704` y `30701163784` terminaron verdes. H4 materializa las fronteras
vacías de `kernel`, `backend` y `engine`: catálogo sellado por campo, builder,
fachada y bucles portables seguros. La primera cobertura eleva la suite a 80
tests ordinarios, más un gate opt-in de asignaciones, y cuatro compile-fail, con
17 tamaños batch y canarios para los tres campos. La matriz completa alcanza
81 tests con todos los features; Miri ejecuta 26 tests de runtime habilitados y
los cuatro doctests sin detectar comportamiento indefinido. Rust 1.89, la
regeneración de los tres artefactos y las 447 pruebas legadas permanecen
correctos.
