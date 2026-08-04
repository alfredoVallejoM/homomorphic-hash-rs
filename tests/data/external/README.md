# Corpus externo opt-in de grafos

Esta suite contrasta el motor con datos reales sin convertir Internet en una
dependencia de `cargo test`. El repositorio conserva únicamente este manifiesto,
la procedencia, las licencias declaradas por upstream y los SHA-256. Los datos
crudos y expandidos viven en `.cache/graph-corpus/`, que está ignorado.

## Reproducción

```bash
python3 tools/fetch_graph_corpus.py
cargo test -p homomorphic-hash-rs --test external_graph_corpus -- --ignored
```

Tras una primera descarga, la integridad y la expansión pueden repetirse sin
red:

```bash
python3 tools/fetch_graph_corpus.py --offline
```

Puede usarse otra caché exportando `MICROFIELD_GRAPH_CORPUS` con la ruta del
directorio `expanded`.

## Cobertura

| Fuente | Estructura cubierta | Gate |
|---|---|---|
| NetworkX Graph Atlas 3.6.1 | 1.253 representantes no isomorfos, hasta 7 vértices | invariancia bajo renumeración y ninguna indistinguibilidad v2 entre representantes |
| TUDataset MUTAG | 188 moléculas | etiquetas de átomo, enlace y clase; invariancia de cada molécula |
| SNAP email-Eu-core | red real dirigida de 1.005 vértices y 25.571 aristas | dirección, departamentos, componentes débiles/SCC y renumeración |
| XGI/Zenodo diseasome | 516 enfermedades y 903 hiperaristas génicas | etiquetas, roles, normalización por incidencias y renumeración |

El corpus no pretende representar literalmente todas las clases posibles de
grafos. Sí fija cuatro familias semánticamente diferentes y permite añadir una
fuente nueva sin alterar el test runner: URL estable, SHA-256, licencia/cita y
un adaptador determinista.

MUTAG y SNAP se mantienen en modo `cache-only`: el usuario debe consultar y
respetar las condiciones de sus fuentes. Diseasome declara CC-BY-4.0. NetworkX
se distribuye bajo BSD-3-Clause y documenta la procedencia del Graph Atlas.
