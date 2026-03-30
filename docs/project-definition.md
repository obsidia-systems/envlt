# envlt — Project Definition Document

> Legacy note (2026): this document is a historical planning artifact in Spanish.
> For current implementation-aligned documentation in English, start with `legacy-project-definition-summary.md`, `spec-alignment.md`, and `architecture.md`.

**Version:** 1.0.0  
**Date:** 2026  
**Status:** Pre-development  

---

## 1. Resumen ejecutivo

**envlt** (pronunciado *"en-vault"*) es una herramienta CLI de código abierto escrita en Rust para la gestión segura de variables de entorno en entornos de desarrollo local. Permite a desarrolladores individuales y equipos pequeños almacenar, cifrar, compartir y regenerar archivos `.env` sin depender de servicios en la nube, suscripciones ni infraestructura externa.

El nombre combina `env` (variables de entorno) y `vault` (bóveda), comunicando su propósito de forma directa y buscable.

---

## 2. El problema

### 2.1 Contexto

Todo proyecto de software moderno requiere variables de entorno: credenciales de base de datos, API keys, tokens de autenticación, URLs de servicios externos. Estas variables son sensibles, cambian entre entornos (local, staging, producción) y deben mantenerse fuera del control de versiones.

### 2.2 Soluciones actuales y sus fallas

| Solución actual                          | Problema                                                                                     |
| ---------------------------------------- | -------------------------------------------------------------------------------------------- |
| Guardar `.env` en el repo (privado)      | Inseguro. Un repo privado no es cifrado. Expone secretos en el historial de Git.             |
| Compartir por Slack / email              | Sin cifrado, sin trazabilidad, difícil de actualizar.                                        |
| Herramientas como Doppler / Infisical    | Requieren cuenta, servidor propio o suscripción. Overkill para un dev solo o equipo pequeño. |
| direnv                                   | Inyecta variables al shell pero no gestiona secretos ni permite compartirlos.                |
| Repositorio de `.env` separado y privado | Desincronización frecuente. Sin cifrado real. No escala.                                     |
| Simplemente no tener `.env` documentado  | El desarrollador nuevo pierde horas preguntando qué variables necesita.                      |

### 2.3 El vacío

No existe una herramienta que sea simultáneamente:

- **Local-first**: sin servidor, sin cuenta, sin internet requerido.
- **Segura**: cifrado real de los secretos, no solo acceso restringido.
- **Simple**: instalable en un comando, usable sin documentación.
- **Portable**: exportable a un compañero o importable en otra máquina.
- **Liviana**: un solo binario, sin runtime, sin dependencias del sistema.

**envlt** ocupa ese vacío.

---

## 3. La solución

### 3.1 Concepto central

envlt introduce el concepto de **vault local**: un archivo cifrado en la máquina del desarrollador que actúa como fuente de verdad para todas las variables de entorno de todos sus proyectos. El archivo `.env` de cada proyecto se convierte en un artefacto desechable y regenerable — no en el lugar donde viven los secretos.

### 3.2 El modelo mental

```
ANTES:
  .env (texto plano en disco) ← fuente de verdad ← riesgo de exposición

DESPUÉS:
  vault.age (cifrado) ← fuente de verdad
       ↓
  .env (generado bajo demanda, efímero)   ← o nunca existe (envlt run)
```

### 3.3 El archivo `.envlt-link`

Cada proyecto tiene un archivo `.envlt-link` en su raíz. Este archivo solo contiene el nombre del proyecto en el vault. Es seguro de commitear — no tiene secretos, no tiene hashes de variables, solo una referencia:

```toml
# .envlt-link
project = "api-payments"
envlt_version = "1.0"
```

Este es el puente entre el repositorio y el vault local del desarrollador.

---

## 4. Casos de uso

### 4.1 Desarrollador individual — múltiples proyectos

María tiene 6 proyectos activos: 3 APIs en Node, 1 app en Python, 1 servicio en Go, 1 proyecto con Docker Compose. Cada uno tiene su propio conjunto de variables. Con envlt:

1. `envlt init` — crea el vault una vez.
2. `envlt add mi-api` — ingesta el `.env` actual al vault.
3. Borra el `.env` del repo o lo agrega al `.gitignore`.
4. En cualquier momento: `envlt run node server.js` para arrancar sin generar archivo.
5. `envlt cloud link icloud` — el vault se sincroniza entre su Mac de escritorio y su laptop automáticamente.

### 4.2 Equipo pequeño sin presupuesto para herramientas

Un equipo de 3 desarrolladores comparte las credenciales de staging:

1. El lead tiene el vault configurado.
2. Ejecuta `envlt export api-staging --only-secrets --out bundle.evlt`.
3. Comparte `bundle.evlt` por Slack con una password acordada por separado.
4. Cada desarrollador ejecuta `envlt import bundle.evlt` y tiene las variables en su vault local.
5. Cuando cambian las credenciales: nuevo export, nuevo bundle.

### 4.3 Incorporación de un nuevo desarrollador

El repositorio ya tiene `.env.example` y `.envlt-link`. El nuevo dev:

1. Instala envlt: `brew install envlt`.
2. Ejecuta `envlt init`.
3. Ejecuta `envlt add --from-example .env.example` — envlt le pregunta interactivamente solo las variables vacías (los secretos reales).
4. En 5 minutos tiene su entorno listo sin preguntar a nadie qué variables necesita.

### 4.4 Generación de secretos seguros

Un desarrollador necesita un JWT secret para su app:

```bash
envlt gen --type jwt-secret --set JWT_SECRET --project api-auth
```

envlt genera un secret de 256 bits criptográficamente seguro, lo guarda directamente en el vault. El valor nunca pasa por el terminal ni por el clipboard.

### 4.5 Rotación de credenciales

Las credenciales de la base de datos cambiaron:

```bash
envlt set DB_PASSWORD=nuevo-valor --project api-payments --secret
```

El vault se actualiza. La próxima vez que alguien ejecute `envlt run` o `envlt use`, obtiene el valor nuevo. Sin editar archivos manualmente.

---

## 5. Arquitectura

### 5.1 Principios de diseño

- **Local-first**: el vault vive en la máquina del usuario. No hay servidor de envlt. La sincronización es responsabilidad del proveedor de nube que el usuario elija.
- **Zero-trust en disco**: nada sensible existe en texto plano en disco. El vault siempre está cifrado. Los `.env` generados son efímeros.
- **Composabilidad Unix**: envlt no reemplaza tu stack — lo envuelve. `envlt run node server.js` sigue siendo Node. `envlt run docker-compose up` sigue siendo Docker Compose.
- **Falla con claridad**: errores descriptivos, sin mensajes genéricos. Si falla el descifrado, envlt dice por qué. Si falta una variable, envlt dice cuál.
- **Sin magia**: el usuario sabe exactamente qué hace envlt en cada paso. Sin side effects ocultos.

### 5.2 Estructura del workspace

**envlt** es un Cargo workspace con tres crates principales:

1. **`envlt-core`** — Library compartida con toda la lógica de vault, crypto, y variables. Reutilizada por CLI y GUI.
2. **`envlt-cli`** — Binary CLI; consume `envlt-core`.
3. **`envlt-bar`** — App nativa de barra de menú con Tauri v2 + Svelte 5; consume `envlt-core`.

Todos comparten el mismo `Cargo.lock`, mismo repositorio. El vault (`~/.envlt/vault.age`) es leído directamente por ambos sin capas de sincronización.

### 5.3 Estructura de módulos

```
envlt/                          ← Workspace root
├── Cargo.toml                  ← Workspace Cargo.lock
├── README.md
├── .github/
│   └── workflows/
│       ├── ci.yml          # Tests en cada PR para todos los crates
│       └── release.yml     # Build multiplataforma y release
│
├── crates/
│   ├── envlt-core/         ← Library compartida (vault, crypto, variables)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs      # Punto de entrada de la library
│   │   │   ├── vault/
│   │   │   │   ├── mod.rs          # Tipos públicos del vault
│   │   │   │   ├── model.rs        # VaultData, Project, Variable, VarType
│   │   │   │   ├── crypto.rs       # Cifrado/descifrado con age
│   │   │   │   ├── store.rs        # Lectura y escritura del vault en disco
│   │   │   │   ├── merge.rs        # Lógica de merge entre vaults
│   │   │   │   └── migration.rs    # Migraciones de formato entre versiones
│   │   │   ├── env/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── parser.rs       # Parser de archivos .env y .env.example
│   │   │   │   ├── writer.rs       # Generador de archivos .env
│   │   │   │   └── injector.rs     # Inyección de vars a procesos hijos
│   │   │   ├── bundle/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── format.rs       # Formato binario .evlt
│   │   │   │   ├── export.rs       # Serialización y cifrado de bundles
│   │   │   │   └── import.rs       # Validación e importación de bundles
│   │   │   ├── cloud/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── provider.rs     # Trait CloudProvider
│   │   │   │   ├── icloud.rs       # Implementación iCloud
│   │   │   │   ├── dropbox.rs      # Implementación Dropbox
│   │   │   │   └── gdrive.rs       # Implementación Google Drive
│   │   │   ├── gen/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── generator.rs    # Generación de secrets con CSPRNG
│   │   │   │   └── types.rs        # Tipos predefinidos: jwt-secret, uuid, api-key...
│   │   │   ├── keychain/
│   │   │   │   ├── mod.rs
│   │   │   │   └── macos.rs        # Integración con macOS Keychain
│   │   │   └── error.rs            # Tipos de error centralizados (thiserror)
│   │   └── tests/
│   │       └── integration/
│   │           ├── vault_ops.rs
│   │           ├── bundle_ops.rs
│   │           ├── gen_ops.rs
│   │           └── cloud_ops.rs
│   │
│   ├── envlt-cli/           ← CLI binary (consume envlt-core)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs             # Entry point — inicializa CLI
│   │       ├── cli/
│   │       │   ├── mod.rs          # Definición de comandos con clap
│   │       │   ├── init.rs         # envlt init
│   │       │   ├── add.rs          # envlt add
│   │       │   ├── run.rs          # envlt run
│   │       │   ├── use_cmd.rs      # envlt use
│   │       │   ├── set.rs          # envlt set
│   │       │   ├── list.rs         # envlt list
│   │       │   ├── export.rs       # envlt export
│   │       │   ├── import.rs       # envlt import
│   │       │   ├── gen.rs          # envlt gen
│   │       │   ├── diff.rs         # envlt diff
│   │       │   ├── cloud.rs        # envlt cloud
│   │       │   ├── auth.rs         # envlt auth
│   │       │   ├── sync.rs         # envlt sync
│   │       │   └── doctor.rs       # envlt doctor
│   │       ├── ui/
│   │       │   ├── mod.rs
│   │       │   ├── prompt.rs       # Prompts interactivos (inquire)
│   │       │   ├── output.rs       # Formateo de output (colored, tablas)
│   │       │   └── progress.rs     # Indicadores de progreso
│   │       └── error.rs            # Adaptador de errores de envlt-core
│   │
│   └── envlt-bar/           ← GUI app (barra de menú con Tauri v2 + Svelte 5)
│       ├── Cargo.toml
│       ├── src-tauri/
│       │   ├── src/
│       │   │   ├── bin/
│       │   │   │   └── main.rs     # Entry point de Tauri
│       │   │   ├── commands.rs     # Comandos que expone Tauri al frontend
│       │   │   └── lib.rs          # Helpers y manejo de estado
│       │   └── Cargo.toml
│       ├── src/
│       │   ├── app.svelte          # Componente raíz
│       │   ├── lib/
│       │   │   ├── components/     # Componentes Svelte reutilizables
│       │   │   ├── stores/         # Svelte stores para estado global
│       │   │   └── utils.ts        # Helpers de TypeScript
│       │   └── routes/             # Rutas de la app (SvelteKit)
│       ├── package.json
│       ├── tailwind.config.js
│       └── tsconfig.json
│
└── docs/
    ├── architecture.md
    ├── security.md
    └── contributing.md
```

### 5.4 Modelo de datos

```rust
// vault/model.rs

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VaultData {
    pub version: u32,                           // Para migraciones futuras
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub projects: HashMap<String, Project>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Project {
    pub name: String,
    pub description: Option<String>,
    pub path: Option<PathBuf>,                  // Path del proyecto en disco
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub variables: HashMap<String, Variable>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Variable {
    pub value: String,
    pub var_type: VarType,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub generated: bool,                        // Si fue generada por envlt gen
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum VarType {
    Secret,   // Cifrado extra, nunca se imprime en claro en terminal
    Config,   // Visible en logs y list, no sensible
    Plain,    // Valor público, puede ir en .env.example con el valor
}
```

### 5.5 Formato del vault en disco

El vault es un archivo TOML serializado con serde y cifrado con age usando passphrase. Reside en `~/.envlt/vault.age`.

```
~/.envlt/
├── vault.age           ← vault principal (cifrado con age)
├── vault.age.bak       ← backup automático pre-rotación
├── config.toml         ← configuración de envlt (no sensible)
└── cloud.toml          ← configuración del proveedor de nube (no sensible)
```

El `config.toml` no cifrado solo contiene preferencias no sensibles:

```toml
[envlt]
version = "1.0"
cloud_provider = "icloud"   # o "dropbox", "gdrive", "none"
vault_path = "~/.envlt/vault.age"
auto_backup = true
backup_retention_days = 30
secret_inference = true     # Infiere VarType por nombre de variable
```

### 5.6 Formato del bundle `.evlt`

Archivo binario portable para exportar proyectos:

```
[4 bytes]  Magic number: 0x454E564C ("ENVL")
[1 byte]   Version del formato
[2 bytes]  Longitud del header JSON
[N bytes]  Header JSON: { "project": "...", "exported_at": "...", "envlt_version": "..." }
[12 bytes] Nonce de ChaCha20-Poly1305
[M bytes]  Payload cifrado: TOML del Project serializado con serde
[16 bytes] Authentication tag de ChaCha20-Poly1305
```

### 5.7 Patrones de diseño aplicados

**Command Pattern** — cada subcomando de la CLI es una struct que implementa un trait `Command`:

```rust
pub trait Command {
    fn execute(&self, ctx: &AppContext) -> Result<(), EnvltError>;
}
```

**Repository Pattern** — el vault se accede siempre a través de `VaultStore`, nunca directamente desde los comandos. Esto permite testear con implementaciones en memoria.

**Provider Pattern** — el sync con servicios de nube se implementa a través del trait `CloudProvider`. Agregar soporte para un nuevo proveedor es implementar el trait, sin tocar la lógica principal.

**Builder Pattern** — la generación de secrets usa un `SecretBuilder` fluido:

```rust
SecretBuilder::new()
    .length(64)
    .charset(Charset::HexLower)
    .build()
```

**Error Wrapping con thiserror** — todos los errores son tipos propios que implementan `std::error::Error`. Sin `unwrap()` en código de producción. Cada error tiene un mensaje de usuario claro y un contexto de debug.

---

## 6. Stack tecnológico

### 6.1 Lenguaje y toolchain

| Componente             | Tecnología        | Justificación                                                          |
| ---------------------- | ----------------- | ---------------------------------------------------------------------- |
| Lenguaje               | Rust 2021 edition | Binario único sin runtime, memoria segura, ecosistema de crypto maduro |
| Package manager        | Cargo             | Estándar de Rust, gestión de dependencias y build integrados           |
| Edición mínima de Rust | 1.75.0 (MSRV)     | Soporte de `async fn` en traits estables                               |

### 6.2 Crates principales

| Crate                           | Versión      | Uso                                                                     |
| ------------------------------- | ------------ | ----------------------------------------------------------------------- |
| `clap`                          | 4.x (derive) | Framework CLI — subcomandos, flags, help automático                     |
| `serde` + `serde_json` + `toml` | latest       | Serialización del vault y configuración                                 |
| `age`                           | 0.10.x       | Cifrado del vault con passphrase (algoritmo X25519 + ChaCha20-Poly1305) |
| `chacha20poly1305`              | 0.10.x       | Cifrado de bundles `.evlt` portátiles                                   |
| `rand`                          | 0.8.x        | CSPRNG para generación de secrets                                       |
| `zxcvbn`                        | 2.x          | Medición de fortaleza de passwords                                      |
| `inquire`                       | 0.7.x        | Prompts interactivos (selección, input, confirmación)                   |
| `colored`                       | 2.x          | Output coloreado en terminal                                            |
| `rpassword`                     | 7.x          | Lectura de passwords sin echo en terminal                               |
| `thiserror`                     | 1.x          | Definición ergonómica de tipos de error propios                         |
| `anyhow`                        | 1.x          | Propagación de errores en el binario principal                          |
| `chrono`                        | 0.4.x        | Timestamps en el vault y bundles                                        |
| `dirs`                          | 5.x          | Rutas del sistema cross-platform (~, home, config dir)                  |
| `tempfile`                      | 3.x          | Archivos temporales seguros en tests                                    |
| `indicatif`                     | 0.17.x       | Barras de progreso para operaciones largas                              |
| `base64`                        | 0.21.x       | Encoding para tokens y api-keys generados                               |
| `uuid`                          | 1.x          | Generación de UUIDs v4                                                  |
| `security-framework`            | 2.x          | Keychain de macOS (feature-gated, solo en macOS)                        |

### 6.3 Testing

| Herramienta  | Uso                                    |
| ------------ | -------------------------------------- |
| `cargo test` | Unit tests e integration tests nativos |
| `assert_cmd` | Tests de la CLI como proceso externo   |
| `predicates` | Assertions sobre output de procesos    |
| `tempfile`   | Vaults temporales aislados por test    |

### 6.4 CI/CD

| Herramienta    | Uso                                                       |
| -------------- | --------------------------------------------------------- |
| GitHub Actions | Pipeline principal                                        |
| `cargo-deny`   | Auditoría de licencias y vulnerabilidades en dependencias |
| `cargo-audit`  | Chequeo de advisories de seguridad                        |
| `clippy`       | Linting estricto de Rust                                  |
| `rustfmt`      | Formateo consistente del código                           |
| `cross`        | Compilación cruzada para targets Linux ARM, Windows       |

### 6.5 Distribución

| Canal                  | Comando del usuario                                    |
| ---------------------- | ------------------------------------------------------ |
| Homebrew (macOS/Linux) | `brew install envlt/tap/envlt`                         |
| crates.io              | `cargo install envlt`                                  |
| GitHub Releases        | Descarga directa de binario para macOS, Linux, Windows |
| Scoop (Windows)        | `scoop install envlt`                                  |

### 6.6 GUI — Fase 5 (opcional)

**Nota:** `envlt-bar` es parte del mismo workspace y comparte `envlt-core` con el CLI. No se distribuye en Fase 1–5; está planeado para versiones futuras (post-v1.0.1).

| Componente    | Tecnología                                   |
| ------------- | -------------------------------------------- |
| Framework     | Tauri v2                                     |
| Frontend      | Svelte 5 + TypeScript                        |
| UI Components | shadcn-svelte                                |
| Estilo        | Tailwind CSS v4                              |
| IPC Backend   | Reutiliza el core del CLI como library crate |
| Distribución  | App nativa macOS (arm64 + x86_64)            |

---

## 7. Seguridad

### 7.1 Modelo de amenazas

envlt protege contra:

- **Acceso no autorizado al vault**: cifrado age con passphrase. Sin la password, el vault es ilegible.
- **Exposición de secretos en disco**: `envlt run` inyecta variables en memoria sin generar archivo. `envlt use` genera un `.env` efímero que el usuario puede borrar manualmente.
- **Exposición en historial de shell**: `envlt set` y `envlt gen --set` nunca imprimen el valor en stdout. Usan `rpassword` para input sin echo.
- **Bundles interceptados**: los bundles `.evlt` están cifrados con una password independiente al vault. Solo quien tenga esa password puede importarlos.
- **Pérdida del vault**: backup automático antes de cualquier operación destructiva (rotate, delete project).

envlt **no** protege contra:

- Un OS comprometido que pueda leer memoria de procesos.
- Un usuario con acceso físico a la máquina y la password del vault.
- Keyloggers que capturen la master password.

Estos escenarios están fuera del scope de una herramienta local-first y son responsabilidad del sistema operativo.

### 7.2 Algoritmos de cifrado

| Uso                                  | Algoritmo                        | Justificación                                            |
| ------------------------------------ | -------------------------------- | -------------------------------------------------------- |
| Vault principal                      | age (X25519 + ChaCha20-Poly1305) | Estándar moderno, auditado, simple de usar correctamente |
| Bundles portátiles                   | ChaCha20-Poly1305                | AEAD — autenticación e integridad en un solo paso        |
| Derivación de clave desde passphrase | scrypt (via age)                 | Resistente a ataques de fuerza bruta con hardware        |
| Generación de secrets                | CSPRNG (rand::thread_rng)        | Criptográficamente seguro, no Math.random()              |

### 7.3 Inferencia de tipo de variable

Por defecto, envlt infiere `VarType::Secret` si el nombre de la variable contiene alguna de las siguientes palabras (case-insensitive): `KEY`, `SECRET`, `PASSWORD`, `PASS`, `TOKEN`, `CREDENTIAL`, `PRIVATE`, `API_KEY`, `AUTH`. El usuario puede sobreescribir la inferencia con flags explícitos.

---

## 8. Comandos de la CLI — Referencia completa

### Inicialización

```bash
envlt init                          # Crea el vault en ~/.envlt/vault.age
envlt doctor                        # Diagnóstico del estado del vault y config
```

### Gestión de proyectos

```bash
envlt add <nombre>                  # Ingesta el .env del directorio actual
envlt add <nombre> --from-example .env.example  # Ingesta desde template
envlt list                          # Lista todos los proyectos en el vault
envlt remove <nombre>               # Elimina un proyecto del vault
envlt rename <nombre> <nuevo>       # Renombra un proyecto
```

### Variables

```bash
envlt set <KEY>=<VALUE> --project <nombre>          # Agrega o actualiza variable
envlt set <KEY>=<VALUE> --project <nombre> --secret # Marca como secret explícitamente
envlt get <KEY> --project <nombre>                  # Lee una variable (secrets ocultos por defecto)
envlt get <KEY> --project <nombre> --reveal         # Muestra el valor del secret
envlt unset <KEY> --project <nombre>                # Elimina una variable
envlt vars <nombre>                                 # Lista variables de un proyecto
```

### Uso del vault

```bash
envlt run <comando>                 # Inyecta vars y ejecuta comando (sin .env en disco)
envlt use [nombre]                  # Genera .env en el directorio actual
envlt use [nombre] --ttl 60        # Genera .env que se borra en 60 segundos
```

### Comparación y diagnóstico

```bash
envlt diff <nombre> --example .env.example  # Compara vault vs .env.example
envlt diff <nombre> <otro-nombre>           # Compara dos proyectos
```

### Exportación e importación

```bash
envlt export <nombre> --out bundle.evlt          # Exporta proyecto completo
envlt export <nombre> --out bundle.evlt --only-secrets  # Solo variables secret
envlt import bundle.evlt                         # Importa bundle al vault local
```

### Sincronización con nube

```bash
envlt cloud link icloud             # Vincula vault a iCloud Drive
envlt cloud link dropbox            # Vincula vault a Dropbox
envlt cloud link gdrive             # Vincula vault a Google Drive
envlt cloud unlink                  # Desvincula y mantiene vault local
envlt cloud status                  # Estado de sincronización
envlt sync --from <ruta>            # Merge manual desde un vault externo
```

### Gestión de la master password

```bash
envlt auth rotate                   # Cambia la master password del vault
envlt auth generate                 # Genera una password fuerte sugerida
envlt auth status                   # Info sobre la password actual
envlt auth keychain enable          # Guarda password en macOS Keychain
envlt auth keychain disable         # Elimina password del Keychain
```

### Generación de secrets

```bash
envlt gen                               # Genera secret con configuración interactiva
envlt gen --len 64 --hex                # 64 caracteres hexadecimales
envlt gen --len 32 --symbols            # 32 caracteres con símbolos
envlt gen --type jwt-secret             # JWT secret de 256 bits (64 hex chars)
envlt gen --type uuid                   # UUID v4
envlt gen --type api-key                # API key en base58, 40 chars
envlt gen --type password               # Password memorable de 4 palabras
envlt gen --type token                  # Token base64url de 48 chars
envlt gen --list-types                  # Lista todos los tipos disponibles
envlt gen --type jwt-secret --set JWT_SECRET --project mi-api  # Genera y guarda
envlt gen --type jwt-secret --set JWT_SECRET --project mi-api --silent  # Sin output
```

---

## 9. Roadmap de desarrollo

### Fase 0 — Aprendizaje de Rust (1–2 semanas)

- Capítulos 1–12 de The Rust Book (doc.rust-lang.org/book)
- Ejercicio minigrep del capítulo 12
- Familiarización con cargo, rustup, rust-analyzer

### Fase 1 — MVP funcional (2–3 semanas)

**Objetivo:** vault local + comandos básicos usables para uno mismo.

- Scaffold del proyecto con clap v4
- Modelo de datos (VaultData, Project, Variable)
- Cifrado y descifrado del vault con age
- Comandos: `init`, `add`, `use`, `list`, `set`, `run`
- Parser de archivos `.env`
- Tests de integración de operaciones básicas

### Fase 2 — Export / Import (1–2 semanas)

**Objetivo:** compartir proyectos con compañeros.

- Formato binario `.evlt`
- Comandos: `export`, `import`
- Cifrado de bundles con ChaCha20-Poly1305
- Tests de roundtrip export/import

### Fase 3 — Features avanzadas de variables (1 semana)

**Objetivo:** tipado de variables y .env.example.

- VarType (Secret, Config, Plain)
- Inferencia automática por nombre
- `add --from-example`
- `diff --example`
- Comando `gen` con todos los tipos

### Fase 4 — Sync con nube (1 semana)

**Objetivo:** sincronización transparente entre máquinas.

- Trait CloudProvider
- Implementaciones: iCloud, Dropbox, Google Drive
- Comandos: `cloud link`, `cloud status`, `sync`
- Detección y resolución de conflictos

### Fase 5 — Pulido y distribución pública (1–2 semanas)

**Objetivo:** que otros puedan instalarlo y usarlo.

- `envlt doctor` para diagnóstico
- GitHub Actions: CI + release multiplataforma
- Homebrew tap
- Publicación en crates.io
- Binario distribuble en GitHub Releases
- README con ejemplos reales
- docs/security.md
- Scoop (Windows) y otros package managers

### Fase 6 — Integración macOS Keychain (1 semana, opcional)

**Objetivo:** almacenar la master password en macOS Keychain.

- Integración con macOS Keychain (security-framework feature-gated)
- Comandos: `auth keychain enable`, `auth keychain disable`
- Tests de integración con Keychain

### Fase 7 — GUI envlt-bar (2–3 semanas, post-v1.0)

**Objetivo:** UX de escritorio para usuarios que evitan terminal.

**Notas:**
- `envlt-bar` es un crate separado en el mismo workspace (`crates/envlt-bar/`)
- Reutiliza `envlt-core` directamente — mismo vault, sin sincronización
- No incluida en v1.0; se distribuye como complemento opcional post-v1.0.1
- App nativa de barra de menú macOS (Tauri v2 + Svelte 5)

**Funcionalidad:**
- Ícono en la barra de estado (top-right macOS)
- Click → popover con lista de proyectos y variables
- Copiar variables al clipboard (con timeout de 30s)
- Generar y regenerar `.env`
- Exportar bundles
- Cambiar proyecto activo
- Acceso a vault settings y password rotation

**Distribución:**
- macOS only (arm64 + x86_64)
- GitHub Releases como `.dmg`
- Homebrew tap: `brew install envlt/tap/envlt-bar` (posterior)

---

## 10. Convenciones de código

### 10.1 Rust

- **MSRV:** Rust 1.75.0
- **Edición:** 2021
- **Formateo:** `rustfmt` con configuración por defecto
- **Linting:** `clippy` con `#![deny(clippy::all)]` en `main.rs`
- **Sin `unwrap()`** en código de producción. Usar `?` y tipos `Result<_, EnvltError>`.
- **Sin `expect()`** salvo en inicialización de la app donde el fallo es irrecuperable, con un mensaje descriptivo.
- **Documentación:** `///` en todos los tipos y funciones públicas del módulo `vault/`.

### 10.2 Commits

Formato: Conventional Commits (conventionalcommits.org)

```
feat(vault): add merge logic for conflicting projects
fix(crypto): handle invalid passphrase gracefully
docs(readme): add quickstart examples
chore(deps): update age to 0.10.1
```

### 10.3 Versionado

Semantic Versioning (semver.org): `MAJOR.MINOR.PATCH`

- MAJOR: cambios incompatibles en el formato del vault o la CLI
- MINOR: features nuevas retrocompatibles
- PATCH: bugfixes

El formato del vault incluye un campo `version: u32` para migraciones automáticas entre versiones mayores.

---

## 11. Estructura del repositorio

```
github.com/usuario/envlt/          ← Workspace Cargo root
├── README.md                       ← Quickstart, instalación, comandos básicos
├── CHANGELOG.md                    ← Historial de cambios por versión
├── CONTRIBUTING.md                 ← Guía para contribuidores
├── LICENSE                         ← MIT
├── Cargo.toml                      ← Workspace root (members = [...])
├── Cargo.lock                      ← Lock file compartido para todos los crates
│
├── crates/
│   ├── envlt-core/                 ← Library compartida
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── vault/
│   │   │   ├── env/
│   │   │   ├── bundle/
│   │   │   ├── cloud/
│   │   │   ├── gen/
│   │   │   ├── keychain/
│   │   │   └── error.rs
│   │   └── tests/
│   │
│   ├── envlt-cli/                  ← CLI binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── cli/
│   │       └── ui/
│   │
│   └── envlt-bar/                  ← GUI app (Tauri v2 + Svelte 5)
│       ├── Cargo.toml
│       ├── package.json
│       ├── src-tauri/
│       └── src/
│
└── docs/
    ├── architecture.md             ← Este documento, en versión técnica
    ├── security.md                 ← Modelo de amenazas y decisiones de seguridad
    ├── commands.md                 ← Referencia completa de comandos
    └── project-definition.md       ← Definición de producto (este archivo)
```

### 11.1 Ventajas del workspace

| Ventaja                              | Cómo se logra                                                                  |
| ------------------------------------ | ------------------------------------------------------------------------------ |
| Vault compartido                     | Ambos binarios (`envlt-cli` y `envlt-bar`) leen `~/.envlt/vault.age`           |
| Versiones sincronizadas              | Un único `Cargo.lock` — CLI y bar siempre usan mismas versiones de dependencias |
| Fácil refactoring                    | Cambios en `envlt-core` se reflejan automáticamente en ambos clientes           |
| Tests centralizados                  | `envlt-core` tiene tests exhaustivos que validan ambas interfaces              |
| Distribución coordinada              | Un único `Cargo.toml` con ambas features; GitHub Actions publica ambos         |
| Sin duplicación de código            | Lógica de vault, crypto, cloud sync vive una sola vez en `envlt-core`         |

**Licencia:** MIT — libre de usar, modificar y distribuir.

---

## 12. Métricas de éxito

| Métrica                               | Objetivo                        |
| ------------------------------------- | ------------------------------- |
| Tiempo de instalación                 | < 30 segundos con brew          |
| Tiempo hasta primer vault funcionando | < 2 minutos desde instalación   |
| Tamaño del binario                    | < 10 MB                         |
| Tiempo de arranque del CLI            | < 50 ms                         |
| Cobertura de tests                    | > 80% en módulos vault y bundle |
| Tiempo de descifrado del vault        | < 500 ms con scrypt estándar    |

---

*Documento generado como especificación pre-desarrollo de envlt v1.0.0.*  
*Este documento debe actualizarse cuando cambien decisiones de arquitectura significativas.*
