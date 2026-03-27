# Uso del MVP actual

Este documento describe cómo usar la implementación disponible hoy de `envlt`.

## Resumen

`envlt` guarda proyectos y variables en un vault cifrado con `age`. El vault vive por defecto en `~/.envlt/vault.age`, con backup en `~/.envlt/vault.age.bak`.

Cada proyecto puede tener un archivo `.envlt-link` en su raíz:

```toml
project = "api-payments"
envlt_version = "1.0"
```

Ese archivo no contiene secretos. Solo sirve para que `envlt` sepa qué proyecto usar desde ese directorio.

## Variables de entorno útiles

- `ENVLT_HOME`: sobreescribe la carpeta base del vault. Útil en tests o entornos aislados.
- `ENVLT_PASSPHRASE`: evita el prompt interactivo y usa esa passphrase directamente.
- `ENVLT_BUNDLE_PASSPHRASE`: evita el prompt interactivo para bundles `.evlt`.
- `ENVLT_GEN_TYPE`: fija el tipo de `envlt gen` cuando usas el modo interactivo básico o automatización.

## Flujo recomendado

### 1. Inicializar el vault

```bash
envlt init
```

Efecto:

- crea el directorio base de `envlt`
- crea `vault.age`
- pide passphrase y confirmación

### 2. Importar un `.env` actual

```bash
envlt add api-payments
```

Opciones:

```bash
envlt add api-payments --file .env.local
envlt add api-payments --project-path /ruta/al/proyecto
envlt add api-payments --from-example .env.example
```

Efecto:

- lee el `.env`
- guarda el proyecto en el vault
- crea `.envlt-link` en el directorio del proyecto

Si usas `--from-example`, `envlt` conserva los valores ya definidos en el template y pide solo los que estén vacíos.

### 3. Listar proyectos

```bash
envlt list
```

### 3.1 Ver variables y tipos

```bash
envlt vars --project api-payments
```

O desde un directorio con `.envlt-link`:

```bash
envlt vars
```

Salida actual:

- muestra nombre de variable
- muestra `VarType`
- enmascara valores `Secret`

### 3.2 Comparar contra `.env.example`

```bash
envlt diff --project api-payments --example .env.example
```

O desde un directorio con `.envlt-link`:

```bash
envlt diff --example .env.example
```

Salida actual:

- claves compartidas entre vault y example
- claves que faltan en el vault
- claves extra que existen solo en el vault

### 3.3 Comparar dos proyectos

```bash
envlt diff --project api-payments api-auth
```

Salida actual:

- claves compartidas entre ambos proyectos
- claves compartidas con valor distinto
- claves solo presentes en el proyecto izquierdo
- claves solo presentes en el proyecto derecho

### 4. Actualizar una variable

```bash
envlt set --project api-payments PORT=4000
envlt set --project api-payments --secret JWT_SECRET=supersecret
envlt set --project api-payments --plain APP_NAME=my-app
```

Si estás dentro de un directorio con `.envlt-link`, también funciona:

```bash
envlt set PORT=4000
```

### 5. Regenerar `.env`

```bash
envlt use --project api-payments
```

También:

```bash
envlt use
envlt use --out .env.local
```

`envlt use` escribe el archivo de salida con las variables actuales del proyecto.

### 6. Ejecutar un comando con variables inyectadas

```bash
envlt run --project api-payments -- node server.js
```

O desde un directorio con `.envlt-link`:

```bash
envlt run -- npm run dev
```

Las variables se inyectan al proceso hijo. No se escribe `.env` en disco para este caso.

### 7. Exportar un proyecto a bundle

```bash
envlt export api-payments --out bundle.evlt
```

Efecto:

- carga el proyecto desde el vault
- serializa el proyecto
- cifra el payload con `ChaCha20-Poly1305`
- escribe un bundle portable `.evlt`

### 8. Importar un bundle

```bash
envlt import bundle.evlt
```

Efecto:

- lee el bundle `.evlt`
- descifra el payload usando la passphrase del bundle
- inserta el proyecto en el vault local

### 9. Generar secrets

```bash
envlt gen --list-types
envlt gen
envlt gen --type jwt-secret
envlt gen --type password
envlt gen --len 64 --hex
envlt gen --len 32 --symbols
envlt gen --type jwt-secret --set JWT_SECRET --project api-payments --silent
```

Flujo actual:

- genera valores seguros con CSPRNG
- si no pasas `--type` ni `--len`, pide el tipo de generador y usa `token` como default
- puede imprimir el valor generado
- puede guardarlo directo en el vault
- si se guarda con `--set`, usa el tipo sugerido por el generador

## Comandos implementados

### `envlt init`

Inicializa el vault cifrado. Falla si ya existe un vault.

### `envlt add <project>`

Importa variables desde un archivo `.env` al vault y crea `.envlt-link`.

Flags:

- `--file <path>`: ruta del `.env` a importar. Default: `.env`
- `--from-example <path>`: importa desde un `.env.example` y solicita los valores faltantes
- `--project-path <path>`: ruta raíz del proyecto a asociar

### `envlt list`

Lista los nombres de proyectos guardados.

### `envlt vars [--project <name>]`

Lista variables del proyecto mostrando:

- nombre
- tipo inferido
- valor visible para `Config` y `Plain`
- valor enmascarado para `Secret`

Si no se pasa `--project`, intenta resolverlo desde `.envlt-link`.

### `envlt diff [--project <name>] --example <path>`

Compara el proyecto del vault contra un `.env.example`.

Reporta:

- claves compartidas
- claves faltantes en el vault
- claves extra en el vault

Si no se pasa `--project`, intenta resolverlo desde `.envlt-link`.

### `envlt diff [--project <name>] <other-project>`

Compara dos proyectos del vault.

Reporta:

- claves compartidas
- claves compartidas con valor distinto
- claves solo presentes en el proyecto izquierdo
- claves solo presentes en el proyecto derecho

### `envlt set [--project <name>] <KEY=VALUE>`

Crea o actualiza una variable del proyecto.

Si no se pasa `--project`, intenta resolverlo desde `.envlt-link` en el directorio actual.

Flags de tipo:

- `--secret`: fuerza `VarType::Secret`
- `--config`: fuerza `VarType::Config`
- `--plain`: fuerza `VarType::Plain`

Si no se pasa ninguno, `envlt` usa la inferencia automática actual.

### `envlt use [--project <name>] [--out <path>]`

Escribe un archivo `.env` desde el vault.

Si no se pasa `--project`, intenta resolverlo desde `.envlt-link`.

### `envlt run [--project <name>] -- <command> [args...]`

Ejecuta un proceso hijo con las variables del proyecto cargadas en su entorno.

Si no se pasa `--project`, intenta resolverlo desde `.envlt-link`.

### `envlt export <project> --out <path>`

Exporta un proyecto del vault a un bundle `.evlt`.

Pide dos secretos distintos:

- passphrase del vault
- passphrase del bundle

Esto permite compartir el bundle sin exponer la passphrase maestra del vault.

### `envlt import <path>`

Importa un bundle `.evlt` al vault local.

Falla si el proyecto ya existe en el vault actual.

### `envlt import <path> --overwrite`

Importa el bundle y reemplaza el snapshot completo del proyecto existente.

Esto es intencionalmente explícito. No hay merge automático en esta etapa.

### `envlt gen`

Genera valores seguros.

Flags soportados hoy:

- `--list-types`: lista los tipos disponibles
- `--type <name>`: tipo a generar
- `--len <n>`: genera una cadena aleatoria de longitud explícita
- `--hex`: usa charset hexadecimal con `--len`
- `--symbols`: usa charset ampliado con símbolos con `--len`
- `--set <KEY>`: guarda el valor generado en una variable del proyecto
- `--project <name>`: proyecto destino para `--set`
- `--silent`: no imprime el valor; útil cuando se guarda directo en el vault

Tipos disponibles hoy:

- `jwt-secret`
- `uuid`
- `api-key`
- `token`
- `password`

Notas:

- si usas `--len`, `envlt` entra en modo configurable y no usa `--type`
- si no pasas `--type` ni `--len`, `envlt gen` entra en modo interactivo básico y pide el tipo
- el modo interactivo básico usa `token` como default y puede automatizarse con `ENVLT_GEN_TYPE`
- el modo configurable actual usa charset alfanumérico por defecto si no pasas `--hex` ni `--symbols`
- `password` genera actualmente una password memorable de 4 palabras separadas por `-`

## Comportamientos importantes

### Backup del vault

Cada vez que `envlt` sobrescribe un vault existente, copia antes el archivo anterior a `vault.age.bak`.

### Parser `.env`

La implementación actual soporta:

- líneas vacías
- comentarios con `#`
- pares `KEY=VALUE`

Todavía no soporta toda la complejidad de shells ni interpolaciones avanzadas.

## Limitaciones actuales

- `VarType` ya puede inspeccionarse y forzarse en `set`, pero todavía falta edición de tipos dedicada o cambios masivos
- no hay ocultado parcial de secretos en salidas
- no hay `doctor`
- no hay sync con nube
- `diff` hoy compara presencia de claves y detecta cambios de valor, pero no imprime todavía un before/after detallado
- `gen` hoy cubre un conjunto inicial de tipos, un modo configurable básico y un modo interactivo inicial, pero no todavía el flujo interactivo completo ni todos los formatos del documento original

## Tipado automático de variables

El modelo interno ya clasifica variables como:

- `Secret`
- `Config`
- `Plain`

En esta etapa:

- `add` y `set` infieren automáticamente `Secret` por nombre
- `add --from-example` también usa esa inferencia para decidir si pedir el valor como secreto
- si no detecta un patrón sensible, la variable queda como `Config`
- `Plain` queda reservado para etapas posteriores como `.env.example` y flujos explícitos

La inferencia actual marca como `Secret` nombres que contienen términos como:

- `KEY`
- `SECRET`
- `PASSWORD`
- `PASS`
- `TOKEN`
- `CREDENTIAL`
- `PRIVATE`
- `API_KEY`
- `AUTH`

## Formato de bundle implementado

El core ya incluye una base interna para el formato binario `.evlt`:

- header versionado
- magic number `ENVL`
- serialización y parseo del contenedor binario
- pruebas de roundtrip y validación básica

El formato `.evlt` ya tiene:

- magic number `ENVL`
- versión del formato
- header JSON con metadatos y `salt` para KDF
- nonce
- ciphertext
- tag de autenticación

La implementación actual usa:

- `scrypt` para derivar la clave desde la passphrase del bundle
- `ChaCha20-Poly1305` para cifrado autenticado del payload

## Verificación local

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
