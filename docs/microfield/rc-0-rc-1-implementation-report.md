# Informe RC.0–RC.1 — superficie soportada y matriz de campos

Fecha: 4 de agosto de 2026.

Estado: completado localmente.

## Resultado

El paquete raíz ya separa la API mantenida de firmas del motor de grafos y del
legado sin cambiar el comportamiento de la configuración predeterminada.
También existe un inventario machine-readable con estado y contrato de cada
capacidad, respaldado por un test público de compilación.

## Features congeladas

| Feature | Contenido | Dependencias internas |
|---|---|---|
| `signatures` | firmas estáticas mantenidas | ninguna sobre grafos/legacy |
| `dynamic-signatures` | firmas sobre `DynField` | `signatures`, `microfield/dynamic` |
| `graph` | análisis y canonización | `signatures` |
| `legacy` | compatibilidad del prototipo | `graph` |
| `dynamic-fields` | alias compatible: runtime + grafo | `dynamic-signatures`, `graph` |
| `crypto_mode` | compatibilidad de fuente sin garantía | `legacy` |

`default = ["legacy", "signatures", "graph"]` conserva la superficie que antes
se compilaba incondicionalmente. Un consumidor nuevo puede excluirla y elegir
solo las firmas.

## Evidencia ejecutable

El inventario congelado está en
`validation/rc/supported-surface-v1.json`. Clasifica campos, firmas, protocolos,
grafos y adapters como `supported`, `conditional`, `experimental`,
`restricted`, `planned` o `legacy-adapter`.

`tests/rc_supported_surface.rs` comprueba:

- schema, estados, contratos e IDs únicos del inventario;
- contrato común de los tres campos binarios mantenidos;
- contrato común de los tres campos primos mantenidos;
- consumo de un GF(2⁹) externo generado;
- identidades nominales distintas;
- disponibilidad de las seis familias estáticas y tracking;
- disponibilidad de las seis familias runtime sin habilitar grafos;
- igualdad de `FieldId` entre el GF(2⁹) generado y su definición runtime.

## Gates ejecutados

| Gate | Resultado |
|---|---|
| `cargo check ... --no-default-features --features signatures --lib` | correcto |
| `cargo check ... --no-default-features --features dynamic-signatures --lib` | correcto |
| `cargo check ... --no-default-features --features graph --lib` | correcto |
| `cargo check ... --no-default-features --features legacy --lib` | correcto |
| `cargo check ... --all-features --lib` | correcto |
| test RC sin features | 3/3 |
| test RC con `signatures` | 4/4 |
| test RC con `dynamic-signatures` | 5/5 |
| test RC con todas las features | 5/5 |
| suite `structural_signatures` | 32/32 |
| suite `graph_g13_g14` | 7/7 |
| `cargo test -p homomorphic-hash-rs --all-features --all-targets` | 450 unitarios y todas las suites/benches mantenidas correctas; 5 gates externos permanecen `ignored` por diseño |
| Clippy signatures/dynamic/all-features lib | sin warnings |

## Decisión

RC.0 queda cerrado: existe inventario, allowlist inicial y separación de
features. RC.1 queda cerrado para la matriz actualmente admitida de campos: el
test enlaza tipos mantenidos, generados y runtime con el contrato público.

La selección automática `SignatureFieldProfile` pertenece a RC.2 porque es una
política de las firmas, no una nueva capacidad matemática de los campos.

## Siguiente paso

RC.2 estabilizará builders, profiles, snapshots y nombres de la API de firmas.
Después RC.3 podrá introducir deltas versionados sin depender de grafos ni del
legado.
