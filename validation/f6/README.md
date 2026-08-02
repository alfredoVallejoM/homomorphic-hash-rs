# Artefactos F6.V

- `manifest.json`: parámetros congelados antes de observar resultados.
- `schema/`: contratos JSON versionados para semántica y rendimiento.
- `corpora/`: corpus matemáticos u oráculos versionados con procedencia.
- `results/semantic-v1.json`: salida semántica determinista regenerable.
- los resultados de rendimiento son artefactos de CI o ficheros locales; no se
  comparan byte a byte entre CPUs.

Regeneración completa del catálogo simple de ocho vértices:

```bash
DOT_SAGE=/tmp/microfield-sage conda run -n laboratorio_np sage \
  tools/sage/generate_f6_graph_corpus.sage
```

El corpus contiene una clase por isomorfismo producida por `nauty_geng` desde
Sage. El runner no confía en la firma de la librería para construir el oráculo.
