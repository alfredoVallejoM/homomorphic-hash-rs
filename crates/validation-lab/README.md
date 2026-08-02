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
```

`semantic` es determinista. `performance` captura hardware y tiempos y nunca se
usa como golden test entre máquinas.
