# Microfield validation lab

Este crate privado y no publicable ejecuta las campañas reproducibles F6.V.
No forma parte de la API de producto: separa los oráculos, corpus, baselines y
resultados científicos del camino crítico de la librería.

```bash
cargo run -p microfield-validation-lab -- semantic \
  --manifest validation/f6/manifest.json \
  --out validation/f6/results/semantic-v1.json

cargo run --release -p microfield-validation-lab -- performance \
  --manifest validation/f6/manifest.json \
  --out /tmp/f6-performance.json

cargo run --release -p microfield-validation-lab -- g11 \
  --manifest validation/f6/manifest.json \
  --out validation/f6/results/g11-v1.json

cargo run --release -p microfield-validation-lab -- g12 \
  --manifest validation/f6/manifest.json \
  --out validation/f6/results/g12-v1.json

cargo run --release -p microfield-validation-lab -- g13-g14 \
  --manifest validation/f6/manifest.json \
  --out validation/f6/results/g13-g14-v1.json
```

`semantic`, `g11`, `g12` y `g13-g14` son deterministas. `g11` fija un split discovery/holdout y
compara los canales de loops/Green sobre el corpus n=8 autenticado.
`g12` compara el matcher pareado contra formas canónicas, relabelings grandes y
los pares adversariales CFI/SRG.
`g13-g14` congela el enrutado por niveles, los positivos exactos verificados y
la equivalencia diferencial de las rutas incremental/fallback.
`performance` captura hardware y tiempos y nunca se usa como golden test entre
máquinas.
