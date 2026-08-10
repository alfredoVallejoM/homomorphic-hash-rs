# C3 X1/X2: corpora, wires, packaging y compatibilidad

Fecha: 2026-08-11. Estado: **X1 ejecutado como validación externa; X2
implementado y ejecutado como Smoke; `Controlled` pendiente**.

Los fingerprints algebraicos evaluados aquí son **no criptográficos**. La
igualdad de fingerprints nunca se presenta como prueba exacta de isomorfismo,
autenticación o resistencia criptográfica a colisiones.

## X1: datos públicos fijados

`tools/fetch_graph_corpus.py` descargó cinco archivos, comprobó sus SHA-256 y
los expandió en la caché ignorada del repositorio. Los tests release opt-in
produjeron cuatro resultados correctos:

- Graph Atlas: 1.253 representantes no isomorfos, invariantes a renumeración y
  sin dos digests v2 iguales entre representantes distintos;
- MUTAG: 188 moléculas, conservando labels de átomo/enlace y el resultado bajo
  renumeración;
- SNAP email-Eu-core: 1.005 vértices y 25.571 incidencias dirigidas,
  conservando departamentos, WCC, SCC y renumeración;
- XGI diseasome: 516 entidades y 903 hiperaristas etiquetadas, preservando
  roles de incidencia y conectividad.

Los tiempos internos de test fueron 0,14 s, 0,05 s, 0,09 s y 0,03 s. Son
mediciones únicas informativas; la primera invocación además compiló el target.
No se usan como cifras publicables. Checksums, tamaños y límites están en
`c3-x1-external-corpus-v1.json`.

## X2: matriz ejecutable

El plan X2 añade 133 celdas y lleva el inventario C3 generado a 3.243. Incluye
once familias de round-trip, seis familias de truncado/corrupción, compilación
de consumidor locked/offline y equivalencia de la fachada legacy multiset.

Los cuatro preflights completaron 19 celdas, 38 workers y 190 observaciones;
19/19 fueron precisas bajo el umbral Smoke. La semianchura relativa mediana fue
2,91 % y la máxima 19,22 %. Todos los checksums fueron estables.

Las once familias restauraron su estado: additive, sequence, bidirectional,
multiset, multi-sequence, multi-multiset, summary tree, fila DB,
reconciliación, DAG y journal. Las seis familias negativas rechazaron tanto un
prefijo truncado como un magic corrupto. Esto amplía la cobertura temporal,
pero no sustituye fuzzing de bytes arbitrarios ni todas las mutaciones de cada
campo del envelope.

El consumidor externo tardó 10,15 s de mediana por check aislado y fue muy
estable. Su memoria real vive en el subproceso Cargo/Rustc, por lo que no está
representada por el contador de asignaciones del worker; una campaña
publicable debe capturar RSS/cgroup del árbol de procesos. La equivalencia
legacy fue allocation-free y byte-idéntica al perfil mantenido
`LegacyAffineEncoderV1`.

## Conclusión

X1 y X2 quedan sin huecos de ejecución. La evidencia externa aumenta la
representatividad, pero no autoriza afirmaciones universales sobre todos los
grafos o dominios. Los bloques restantes antes de cerrar el preflight C3 son
D2/PostgreSQL y los huecos específicos de backend/reference de F1/F2.
