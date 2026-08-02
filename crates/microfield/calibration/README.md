# Calibración versionada

Este directorio separa disponibilidad ISA de elegibilidad automática.
`selection-table-v1.csv` es la decisión compilada en el runtime;
`profiles/` contiene evidencia Criterion normalizada y `schema-v1.json` fija
su contrato.

Una fila `automatic_selection=false` no deshabilita el backend: permite
forzarlo después de una detección real de CPU, pero impide que `Auto`,
`LowLatency` o `Throughput` lo elijan sin consentimiento.

## Regla conservadora

- mejora mínima: 20 % usando el peor extremo compatible de los intervalos;
- sobrecoste máximo de `Engine` frente al kernel: 3 % en lotes grandes;
- packing incluido cuando el backend lo requiere;
- VPCLMUL necesita perfiles favorables de dos familias x86-64 distintas;
- PMULL necesita perfiles favorables de dos familias AArch64 distintas;
- QEMU nunca constituye evidencia de rendimiento.

La tabla v1 mantiene PCLMUL automático desde un elemento y conserva VPCLMUL y
PMULL como estrategias explícitas. `none` en `minimum_batch` significa que no
se ha certificado ninguna región de selección automática; el runtime lo
representa internamente mediante `usize::MAX` solo como metadata.

## Captura

```text
crates/microfield/tools/capture_calibration.sh /tmp/microfield-calibration
crates/microfield/tools/audit_calibration.sh
```

La captura conserva entorno, comando, `lscpu`, estimadores Criterion y SHA-256.
No modifica la tabla de selección. Promover una medida exige revisión humana,
actualización de la tabla, pruebas del selector y un ADR.
