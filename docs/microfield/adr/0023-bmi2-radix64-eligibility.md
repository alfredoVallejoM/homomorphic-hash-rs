# ADR 0023 — Elegibilidad BMI2 para Montgomery radix 64

## Estado

Aceptada el 2 de agosto de 2026 como corrección final de Fase 4.

## Contexto

La primera estrategia BMI2 estaba ligada directamente a `Fp256GenericV1`. El
producto ya usaba `MULX`, pero la forma
concreta `[u64; 4] -> [u64; 8]` impedía reutilizar el adapter para futuros
campos de 64, 128 o 192 bits. Además, la propagación de carry y la corrección
Montgomery conservaban control dependiente de valores, de modo que una primera
revisión rebajó prudentemente la metadata a `DataDependent`.

Una anchura almacenada de `N * 64` bits demuestra compatibilidad estructural
con `MULX`; no demuestra una mejora de rendimiento. Antes de la reescritura,
`Fp256GenericV1` fue el contraejemplo medido: BMI2 resultó entre 30 % y 64 %
más lento que portable en la CPU de referencia.

## Decisión

Se separan tres conceptos:

1. **Representabilidad:** un tipo generado implementa el puerto público-oculto
   `VerifiedPrimeMontgomery64Field<N, 2N>` cuando almacena `N` limbs de 64 bits
   y aporta módulo, inversa Montgomery, extracción y reconstrucción de limbs.
2. **Elegibilidad ISA:** `VerifiedPrimeIsaStrategy<F, N, 2N>` verifica
   estáticamente representación, número de limbs, reducción y metadata antes
   de construir dentro de Microfield un `KernelSet<F>` monomorfizado. El código
   externo no recibe intrinsics ni punteros de función.
3. **Promoción automática:** continúa siendo una decisión por campo y región,
   ligada a evidencia reproducible. La mera presencia de BMI2 o una anchura
   múltiplo de 64 no activa el backend.

El constructor ancho BMI2 deja de contener el número cuatro: opera sobre los
parámetros const `LIMBS` y `WIDE_LIMBS`. Sus pruebas cubren productos de 1, 2,
3 y 4 limbs —64, 128, 192 y 256 bits— mediante todos los pares de bits de base,
fronteras y muestras deterministas. `Fp256GenericV1` es la primera
implementación mantenida del contrato; los tipos externos de Fase 5 podrán
implementar el mismo puerto público-oculto mediante codegen sin cambiar
`Engine` ni la API estable.

Microfield es dueño del producto ancho, la suma modular y REDC. Cada fila del
producto ejecuta exactamente `N` multiplicaciones y deposita un único carry;
REDC ejecuta `N` filas, cada una con `N` productos y un barrido completo del
sufijo restante. La corrección final resta siempre el módulo y selecciona con
una máscara opaca al optimizador; el artefacto x86-64 auditado no contiene un
salto condicional por valor. Esta frontera impide que un campo externo introduzca una
reducción variable dentro de un kernel anunciado como fijo.

Un test de consumidor define un campo externo `Fp17` de un limb, construye
`PortableStrategy` y `VerifiedPrimeIsaStrategy` únicamente mediante API
público-oculta, fuerza BMI2 y comprueba exhaustivamente sus 289 pares para suma
y producto frente al portable. Así se demuestra que el puente no depende de
que el tipo viva dentro de Microfield.

La metadata BMI2 se eleva finalmente a `ScheduleKind::Fixed` y conserva
`automatic_selection = false`. `ExecutionPolicy::FixedSchedule` acepta el
backend si la CPU posee BMI2. Esta clasificación certifica control y número de
operaciones independientes del valor; no promete por sí sola tiempo constante
del sistema completo ni resistencia frente a todos los canales laterales.

## Consecuencias

- Todo campo Montgomery radix 64 puede obtener un **candidato** BMI2 sin
  duplicar el algoritmo por anchura.
- Un primo de 191 o 255 bits también puede ser elegible si su representación
  usa 3 o 4 limbs; la condición pertenece al radix almacenado, no al número
  exacto de bits del módulo.
- Campos canónicos como Goldilocks y campos binarios no entran por accidente
  en este adapter.
- Un campo nuevo permanece portable hasta que su calibración demuestre una
  región favorable. Esto preserva estabilidad y rendimiento por defecto.
- Una futura ruta `MULX + ADCX/ADOX` o CIOS integrado podrá sustituir el núcleo
  sin modificar los tipos públicos ni los catálogos consumidores.

## Alternativa rechazada

Seleccionar BMI2 automáticamente para toda anchura múltiplo de 64 confunde
compatibilidad con velocidad. Tras la reescritura fija, BMI2 queda mucho más
cerca de portable, pero el build finalmente auditado todavía pierde entre
aproximadamente 2 % y 7 % en los tamaños representativos. La promoción global también impediría comparar
reducciones especializadas —por ejemplo Barrett o Solinas— que pueden ganar a
Montgomery aunque la CPU disponga de `MULX`.
