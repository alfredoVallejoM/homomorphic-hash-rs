# C3-C0: campaña semántica exhaustiva local

Fecha: 2026-08-11. Estado: **gate semántico aprobado; 221.342 casos/controles
deterministas y 15.000 ejecuciones fuzz sin hallazgos**.

Las firmas y resúmenes de Algesum son construcciones algebraicas **no
criptográficas**. Esta campaña demuestra contratos y detecta divergencias; no
demuestra autenticación ni resistencia criptográfica a colisiones.

## Evidencia determinista

El runner release reejecutó `validation/f6/manifest.json` y escribió un
artefacto nuevo bajo `validation/benchmarks/runs/c3-c0-semantic-v1`. JSON y CSV
resultaron byte-idénticos a la evidencia F6 congelada.

| Familia | Casos/controles |
|---|---:|
| ecuaciones metamórficas de firmas | 145.636 |
| pares exhaustivos de reconciliación | 63.232 |
| representantes de grafos no isomorfos | 12.346 |
| invariancia por reetiquetado | 128 |
| **total contabilizado** | **221.342** |

El total supera 18,4 veces el gate C3 de 12.000 casos. Se conservaron las
colisiones algebraicas esperadas: son una propiedad conocida de resúmenes de
dimensión finita, no fallos que deban ocultarse.

## Propiedades y modelos

Se ejecutaron 22 tests dirigidos sin fallos:

- nueve contratos diferenciales y leyes de campos finitos;
- dos tests que obligan a cada capacidad admitida a tener evidencia concreta;
- dos propiedades de canonización, DAG y parsers de grafos;
- nueve máquinas de estado para firmas, deltas, journals, árboles y DB.

Esto enlaza C3-C0 con las 15 suites del ledger en vez de contar únicamente el
gran corpus de grafos.

## Fuzz y sanitización

Los tres targets bloqueados ejecutaron 5.000 entradas cada uno sobre copias
desechables del corpus:

- manifests de campos, hasta 65.536 bytes;
- wires estructurales, deltas y DB, hasta 4.096 bytes;
- wires exactos de grafos y DAG, hasta 4.096 bytes.

No hubo crashes, timeouts ni hallazgos de AddressSanitizer. LeakSanitizer se
desactivó con `detect_leaks=0`, igual que en CI, porque el sandbox usa `ptrace` y
LSAN termina artificialmente en ese entorno. La primera ejecución de manifests
alcanzó sus 5.000 entradas antes de ese error de infraestructura; se descartó y
se repitió completa con la configuración mantenida, por lo que sólo se cuentan
15.000 ejecuciones aceptadas.

## Dictamen

C3-C0 queda cerrado para la superficie actual: evidencia reproducible, cero
discrepancias semánticas abiertas y fuzz local limpio. Este resultado no cierra
C3-Scaling Controlled ni C3-Systems soak, que responden preguntas distintas.
