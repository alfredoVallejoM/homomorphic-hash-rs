# C3: consolidación de preflights y campaña restante

Fecha: 2026-08-11. Estado: **preflights y C3-C0 terminados en el host local; la
campaña publicable `Controlled` y el soak de sistemas requieren la
infraestructura indicada abajo**.

Algesum estudia resúmenes y firmas algebraicas homomórficas **no
criptográficas**. Ninguna igualdad de resumen implica autenticación, seguridad
adversaria o resistencia criptográfica a colisiones.

## Resultado consolidado

Los shards P0, F1–F4, S1–S3, T1, R1, D1, G1–G2 y X2 generan **3.609 celdas**
de publicación. El preflight local ejecutó 229 celdas o calibraciones mediante
485 procesos independientes y conservó 2.770 observaciones. No apareció una
divergencia semántica ni un checksum inestable. Las celdas inicialmente
imprecisas se conservaron; las variantes nuevas de cierre F1/F2 y T1 obtuvieron
al menos una medición precisa tras calibración.

| Bloque | Celdas publicables | Preflight | Resultado principal |
|---|---:|---:|---|
| P0 F1/F2 base | 498 | 30 | 23 precisas; 7 inconclusas conservadas |
| F3–S3 | 2.112 | 56 | 56/56 precisas; batch, runtime, firmas, deltas y journals |
| T1/R1/D1 | 374 | 17 | 16/16 variantes precisas tras calibrar restore |
| G1/G2 | 126 | 17 | 17/17 precisas; 7 exactas y 2 inconclusas por presupuesto esperadas |
| X2 | 133 | 19 | 19/19 precisas; wires, corrupción, packaging y legacy |
| cierre F1/F2 | 366 | 90 | todas las variantes con ejecución precisa; referencias independientes |

X1 fijó cinco fuentes públicas y pasó cuatro verticales: 1.253 grafos del
atlas, 188 moléculas MUTAG, una red dirigida de 1.005 vértices/25.571
incidencias y un hipergrafo de 516 entidades/903 hiperedges. D2 observó
exactamente 1.888 commits PostgreSQL, concurrencia de 1 a 32 clientes, drenaje
2×/5×, migración compatible, rechazo y rollback incompatible, y recuperación
byte-exacta después de reiniciar el servidor.

## Conclusiones que sí permite el preflight

- La superficie declarada dispone de workloads ejecutables salvo el soak
  sostenido D2; F1/F2 ya contrastan rutas optimizadas con referencias
  independientes y reducción prima con `BigUint`.
- Los contratos algebraicos, round-trips, fallos deliberados y equivalencias
  incremental/rebuild cubiertos no mostraron errores.
- PCLMUL/VPCLMUL ofrecen una señal local fuerte en campos binarios, pero una
  sola máquina Smoke no autoriza cambiar la selección automática.
- Fp251 AVX2 y Fp256 BMI2 fueron más lentos en los cruces locales; esto respalda
  mantener umbral/uso explícito. Goldilocks AVX2 mostró sólo una mejora cercana
  al 4 % y necesita réplica multihost.
- La exactitud de grafos sigue dependiendo de comparación/canonización exacta;
  los fingerprints son filtros, no pruebas de isomorfismo.
- PostgreSQL es funcionalmente coherente en el preflight, pero la revisión
  totalmente ordenada empieza a limitar throughput entre 16 y 32 writers.

## Análisis restantes, en orden obligatorio

1. **C3-C0 Semantic — completado:** 221.342 casos/controles deterministas, 22
   tests de propiedades/modelos y 15.000 ejecuciones fuzz sin hallazgos.
2. **C3-Scaling Controlled:** ejecutar las 3.609 celdas con al menos 30 procesos
   por celda (**108.270 procesos por host**) desde release, árbol limpio,
   afinidad fija, frecuencias visibles y sin throttling o swap invalidante.
3. **C3-R multihost:** repetir claims y fronteras en Intel x86-64, AMD x86-64 y
   AArch64. El mínimo teórico completo son 324.810 procesos entre tres hosts,
   antes de recalibraciones.
4. **C3-Systems:** completar 150–300 escenarios, consumidor de replicación,
   writers/readers separados, PostgreSQL en al menos dos versiones, fault
   injection, 64–256 clientes, steady states de 15 min y 1 h y soak de 8 h.
5. **C3-P:** regenerar tablas desde raw, revisar resultados adversarialmente,
   declarar negativos/inconclusos y emitir el dictamen go/no-go.

Estos trabajos no se deben lanzar en este host compartido atribuyéndoles la
etiqueta `Controlled`: las variables de attestación del harness certifican
condiciones reales, no las crean. Ejecutarlos aquí produciría horas de datos
que el propio protocolo prohíbe usar como evidencia publicable.

## Gate actual

Los gates de implementación/preflight y C3-C0 quedan aprobados. El gate de
publicación permanece cerrado por réplica Controlled Intel/AMD/AArch64 y D2 soak. El
estado machine-readable se conserva en
`validation/benchmarks/runs/c3-preflight-consolidated-status-v1.json`.
