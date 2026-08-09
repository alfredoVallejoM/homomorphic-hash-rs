# RC.10 — informe del dictamen reproducible

Fecha: 9 de agosto de 2026.

Estado: implementación y validación local completas. El dictamen local es
`Conditional`; `ReadyForInternalUse` solo puede obtenerse en CI desde el mismo
commit limpio, con RC.7–RC.9 verdes y evidencia x86-64/AArch64.

## Contrato

`validation/rc/decision-manifest-v1.json` congela la campaña
`internal-rc-v1`, sus inputs, arquitecturas requeridas, features, corpus y
limitaciones conocidas. `microfield-validation-lab rc10-decision` genera
`microfield-rc10-decision-report-v1` con:

- commit, estado limpio/sucio y toolchains;
- hardware observado por los informes RC.8 y RC.9;
- matrices de campos y firmas y recuento por clasificación;
- manifiestos SHA-256 de los corpus de decisión;
- gates semánticos, de capacidad e integración externa;
- limitaciones que bloquean uso interno o publicación externa;
- una decisión final cerrada.

La precedencia es fail-closed:

| Evidencia | Decisión |
|---|---|
| algún gate `Fail` | `NotReady` |
| ningún fallo y algún gate `Missing` | `Conditional` |
| todos los gates `Pass` | `ReadyForInternalUse` |

La herramienta rechaza schemas desconocidos y más de un informe para la misma
arquitectura. La falta de x86-64 o AArch64 no se interpreta como éxito.

## Resultado local

La ejecución local verificó la matriz RC.7, el inventario RC.9, los SLO RC.8
y el consumidor RC.9 disponibles en x86-64. El resultado esperado y obtenido
es `Conditional` porque concurren condiciones deliberadamente no simuladas:

- el árbol contiene el cambio que se está evaluando y por tanto no es un
  commit limpio;
- no existe un runner AArch64 local equivalente;
- los gates requeridos de GitHub todavía no pertenecen al commit publicado.

Esto no es un fallo de corrección. Es la forma prevista de impedir que una
ejecución parcial se presente como cierre RC.

## Gate remoto

El job requerido `RC.10 reproducible go-no-go` depende de todos los jobs de
corrección, properties, fuzz, features, MSRV, generación determinista, Miri,
ASan/ISA, F6.V, capacidad y consumidor externo. Descarga los artifacts RC.8 y
RC.9 de x86-64 y AArch64, genera `rc10-decision.json` y exige literalmente:

```text
.final_decision == "ReadyForInternalUse"
```

El artifact resultante es la autoridad para ese commit. Si un gate previo
falla, el job no puede crear una aprobación independiente.

## Alcance del resultado

`ReadyForInternalUse` habilita consumo interno bajo las limitaciones
versionadas. En particular, las firmas homomórficas siguen siendo resúmenes
algebraicos **no criptográficos**: no autentican, no prueban pertenencia y no
autorizan retiradas. La fuente exacta continúa siendo la autoridad.

La decisión tampoco autoriza publicación externa. Licencia, política de
seguridad, changelog, advisories/SBOM, semver público y una fuente publicable
para `microfield` permanecen como workstream separado y están marcados como
bloqueo de publicación, no de uso interno condicionado.
