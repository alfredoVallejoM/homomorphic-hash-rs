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

La revisión del 31 de julio amplió inicialmente la suite a 49 tests y añadió límites duros
de recursos, ensayo independiente para los 247 candidatos mónicos de grados
2–8, contratos de CLI, planes de inversión, consistencia cruzada de artefactos
y casos adversariales de filesystem. El runtime sin features supera además 9
tests bajo Miri. La matriz completa de Microfield pasa con Rust estable 1.93.1,
Clippy y rustdoc con warnings denegados. Los 62 tests y el runtime `no_std`
superan también el MSRV 1.89.
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
estrategias sin implementación y define la matriz CI. La validación Sage está
cerrada; únicamente permanece pendiente la primera observación remota del
workflow.
