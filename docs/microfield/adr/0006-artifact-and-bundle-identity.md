# ADR 0006 — Identidad de artefacto e integridad de bundle

**Estado:** aceptado.

## Contexto

`ArtifactId` identifica una representación, pero el nombre de presentación
cambia varios ficheros sin alterar esa representación. Usarlo como hash del
directorio habría mezclado compatibilidad técnica e integridad byte a byte.

## Decisión

Se mantienen tres value objects independientes:

1. `FieldId`: semántica matemática y encoding canónico.
2. `ArtifactId`: `FieldId`, versión del generador, versión del IR, familia
   target y build normalizado.
3. `ArtifactBundleDigest`: contenido exacto publicado.

El bundle contiene seis payloads y un séptimo fichero `bundle.json`. Para cada
payload, ordenado por ruta, este manifiesto registra:

```json
{"path":"certificate.json","bytes":530,"sha256":"..."}
```

El digest se calcula así:

```text
ArtifactBundleDigest =
  SHA-256("microfield:artifact-bundle:v1\0" || canonical_file_list_json)
```

`bundle.json` no se incluye en su propia lista para evitar una definición
circular. Su contenido se regenera a partir de los seis payloads.

## Consecuencias

- Renombrar un campo conserva `FieldId` y `ArtifactId`, pero cambia
  `ArtifactBundleDigest`.
- Alterar un byte, una ruta o una longitud cambia el digest del bundle.
- El digest comprueba integridad, no autenticidad; una firma futura sería otro
  contrato.

