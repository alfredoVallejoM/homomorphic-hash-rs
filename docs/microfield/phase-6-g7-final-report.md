# Informe F6.G7 — perfil global v2 y validación multiformato

Fecha: 2 de agosto de 2026.

## Corrección de rumbo

La Fase 6 se reabrió porque detectar que `C6` y `C3 ⊔ C3` colisionaban no era
suficiente. Una propuesta rápida útil debía separarlos antes de recurrir a una
búsqueda exponencial. F6.G7 corrige esa carencia sin falsear la naturaleza de
las firmas: v2 discrimina mucho más, pero una igualdad continúa sin probar
isomorfismo.

## Flujo entregado

```text
firma algebraica local v1 + SHA local
                 |
                 v
perfil global exacto (componentes, SCC, labels, relaciones, grados)
                 |
          alta regularidad
                 v
motivos acotados (triángulos y K4)
                 |
         igualdad persistente
                 v
canonización exacta por componentes y presupuesto
```

`FastGraphSignature` v1 no cambia. La nueva fachada
`analyze_discriminating(graph, policy)` devuelve:

- `GraphDiscriminationId`, que liga schema, `GraphSignatureId` y política;
- `HybridGraphAnalysis`, manteniendo el canal homomórfico;
- `GlobalGraphProfile`, cuya igualdad usa la serialización exacta completa;
- `BoundedMotifProfile`, con admisión y trabajo auditables;
- `GraphDiscriminationDigest`, como índice combinado, no como prueba;
- consejo de escalado explícito.

`compare` solo produce `Different` o `Indistinguishable` y rechaza perfiles
incompatibles.

## Garantías y complejidad

El perfil global realiza recorridos CSR iterativos, sin recursión ni `unsafe`.
Agrupa SCC por componente en tiempo lineal y ordena registros canónicos
internados. Para tamaño total de descriptor `S`, su coste es
`O(V + I + S log S)` y memoria `O(V + I + S)`.

Los motivos se calculan sobre el soporte simple no dirigido. La admisión usa
`sum C(deg(v),2) + C(deg(v),3)`, una cota independiente de la numeración. Si
supera `max_motif_work`, no se expone ningún resultado parcial.

La canonización exacta reconoce primero componentes débiles. Cada subgrafo se
certifica con el presupuesto global restante y las formas se ordenan antes de
construir el orden total. Un fallo de cualquier componente devuelve únicamente
`BudgetExhausted`.

## Casos adversariales corregidos

| Par | v1 local/híbrido | global v2 | motivos |
|---|---|---|---|
| `C6` / `C3 ⊔ C3` | indistinguible | 1 frente a 2 componentes | no necesario |
| Shrikhande / torres 4x4 | indistinguible | indistinguible | 32 triángulos ambos; 0 frente a 8 `K4` |

La descomposición exacta se prueba además con `C3 ⊔ C3 ⊔ C4`, incluida una
renumeración que mezcla sus vértices y el agotamiento transaccional del
presupuesto.

## Corpus externo

El manifiesto fija cinco artefactos de cuatro fuentes:

- Graph Atlas de NetworkX 3.6.1: 1.253 clases no isomorfas hasta siete vértices;
- MUTAG/TUDataset: 188 moléculas con átomo, enlace y clase;
- SNAP email-Eu-core: 1.005 nodos, 25.571 aristas dirigidas y 42 departamentos;
- diseasome XGI/Zenodo: 516 enfermedades y 903 hiperaristas génicas.

El fetcher valida SHA-256, descarga atómicamente, impide path traversal del ZIP
y puede repetir expansión offline. Las cuatro suites pasan bajo renumeración.
En el Graph Atlas no queda ningún digest v2 repetido entre representantes no
isomorfos. Datos, licencias y comandos están en
[`tests/data/external/README.md`](../../tests/data/external/README.md).

## Rendimiento local

Criterion release `--quick`, F251 K=3/R=4, ciclo homogéneo:

| Ruta | 1.024 vértices | 16.384 | 131.072 |
|---|---:|---:|---:|
| perfil global exacto | 138–144 µs | 2,45–2,54 ms | 22,87–23,15 ms |
| baseline híbrido local | 1,003–1,010 ms | 15,89–15,93 ms | 132,6–136,1 ms |
| discriminador v2 global | 1,22–1,24 ms | 19,99–20,26 ms | 162,6–179,0 ms |
| discriminador v2 adaptativo | 1,26–1,28 ms | 19,52–20,12 ms | 166,4–172,8 ms |

El cambio de registros byte-copiados a IDs canónicos internados redujo el
perfil global de 67 ms a unos 23 ms a 131.072 vértices. El coste completo está
dominado por la firma/histogramas locales existentes; frente a esa baseline,
la protección global añade aproximadamente 25–40 ms en el caso grande. El
motivo sobre ciclos añade poco. Estas cifras son del host de desarrollo, no
SLA.

## Estado

F6.G7 resuelve el defecto práctico que obligó a reabrir la fase. La ruta v1 se
mantiene para composición y latencia mínima; v2 pasa a ser la recomendada para
discriminación de grafos generales. La canonización exacta conserva su papel de
certificación final, no de filtro primario.
