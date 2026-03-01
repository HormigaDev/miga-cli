# miga — Referencia Técnica

Este documento describe la arquitectura interna de `miga` para contribuidores
y mantenedores.

---

## Tabla de Contenidos

- [Arquitectura general](#arquitectura-general)
- [Punto de entrada del binario](#punto-de-entrada-del-binario)
- [Capa CLI](#capa-cli)
- [Comandos](#comandos)
- [Pipeline del compilador](#pipeline-del-compilador)
- [Protocolo del registro](#protocolo-del-registro)
- [Utilidades compartidas](#utilidades-compartidas)
- [Estructura de archivos](#estructura-de-archivos)
- [Estructuras de datos clave](#estructuras-de-datos-clave)

---

## Arquitectura general

```
┌─────────────────────────────────────────────────┐
│                  binario miga                   │
│                                                 │
│  main.rs  ──►  cli.rs (clap)                    │
│                   │                             │
│          ┌────────┴─────────────────────────┐   │
│          │        commands/                 │   │
│          │  init · add · fetch · run ·      │   │
│          │  build · remove                  │   │
│          └────┬───────────────┬─────────────┘   │
│               │               │                 │
│         compiler/        registry/              │
│         (pipeline oxc)   (manifiesto + HTTP)    │
│               │               │                 │
│          utils/ ─────────────────────────────── │
│          fs · json · npm · env · builder ·      │
│          output · tsconfig · net · project      │
└─────────────────────────────────────────────────┘
```

Todos los comandos devuelven `anyhow::Result<()>`. Los errores se propagan hasta `main.rs`, donde
una única llamada a `output::error()` los formatea y termina el proceso.

---

## Punto de entrada del binario

**`src/main.rs`**

1. Parsea la CLI con `Cli::parse()` (derive de clap).
2. Hace match sobre `Commands` y delega al `commands::*::run()` correspondiente.
3. En caso de error: imprime un mensaje formateado mediante `utils::output::error()` y termina
   con código 1.

---

## Capa CLI

**`src/cli.rs`**

Define `Cli` y `Commands` usando los macros derive de clap.

| Subcomando | Variante del struct                  |
| ---------- | ------------------------------------ |
| `init`     | `Commands::Init { namespace, name }` |
| `add`      | `Commands::Add { packages }`         |
| `fetch`    | `Commands::Fetch { modules }`        |
| `run`      | `Commands::Run { obfuscate }`        |
| `build`    | `Commands::Build { obfuscate }`      |
| `remove`   | `Commands::Remove { module }`        |

---

## Comandos

### `init` (`src/commands/init.rs`)

1. Recopila metadatos de forma interactiva mediante `dialoguer::Input`.
2. Valida el namespace (minúsculas, sin espacios ni dos puntos) y el nombre.
3. Crea el directorio del proyecto y lo establece como directorio de trabajo.
4. Genera UUIDs nuevos para todas las entradas del manifiesto.
5. Escribe los árboles de directorios de BP y RP, manifiestos, scripts de punto de entrada,
   archivos de licencia y plantillas estáticas.
6. Escribe `.miga/miga.json` (manifiesto del proyecto) y `.miga/modules.lock`.
7. Llama a `commands::add::run()` para instalar la versión elegida de `@minecraft/server`.

El **contenido estático** se define como cadenas `const` al final del archivo.
El **contenido dinámico** (manifiestos, licencias, README) es generado por funciones auxiliares
dedicadas que reciben únicamente lo que necesitan.

---

### `add` (`src/commands/add.rs`)

Descarga paquetes de tipos TypeScript desde el registro de npm.

1. `utils::net::is_online()` — verificación de conectividad.
2. `utils::project::require_initialized()` — confirma que existe `.miga/`.
3. Por cada especificación de paquete: llama a `utils::npm::fetch_types()`.
4. Registra `nombre → versión` en `.miga/miga.json` bajo `externals`.
5. Actualiza el bloque de dependencias de `behavior/manifest.json` mediante
   `sync_behavior_manifest()`.

---

### `fetch` (`src/commands/fetch.rs`)

Instala módulos desde el registro de miga.

1. Verificación de conectividad.
2. `utils::project::require_initialized()`.
3. Por cada nombre de módulo: resuelve el árbol completo de dependencias mediante
   `registry::resolve_dependencies()`.
4. Descarga el archivo `.tar.gz` y lo extrae.
5. Registra el módulo en `.miga/modules.lock`.

---

### `run` (`src/commands/run.rs`)

Bucle de vigilancia con hot-reload.

1. `utils::builder::build_project()` — compilación y despliegue inicial.
2. `notify::RecommendedWatcher` vigila `behavior/scripts/`.
3. Ante cualquier evento de cambio: vuelve a ejecutar `build_project()`.

---

### `build` (`src/commands/build.rs`)

Compilación completa y empaquetado.

1. `utils::builder::build_project()` — compilación de TypeScript → JS + despliegue.
2. Sincroniza la versión desde `.miga/miga.json` en ambos manifiestos.
3. Se limpia `dist/`, luego `zip` empaqueta el BP y el RP en archivos `.mcpack`
   y combina ambos en un `.mcaddon`.

---

### `remove` (`src/commands/remove.rs`)

Elimina un módulo del registro.

1. `utils::project::require_initialized()`.
2. Lee el archivo de bloqueo; verifica que el módulo esté instalado.
3. Elimina los archivos del módulo del disco (`behavior/scripts/` y
   `.miga_modules/`).
4. Elimina la arista de dependencia de `behavior/manifest.json`.
5. Escribe el archivo de bloqueo actualizado.

---

## Pipeline del compilador

**`src/compiler/mod.rs`**

Usa la familia de crates de [oxc](https://oxc.rs/) para compilación de TypeScript sin Node.js.

```
fuente .ts
    │
    ▼
oxc_parser::Parser::parse()          — produce el AST
    │
    ▼
oxc_semantic::SemanticBuilder        — análisis de scope y bindings, produce
                                       el Scoping que se pasa al Transformer
    │
    ▼
oxc_transformer::Transformer         — TypeScript → JS ES2020
    │
    ▼
(opcional) oxc_minifier::Minifier    — eliminación de código muerto + mangling
    │
    ▼
oxc_codegen::Codegen                 — emite la cadena JS final
```

**Reescritura de imports**: los imports relativos con `./` y `../` tienen su extensión `.ts`
reemplazada por `.js`. Los especificadores `minecraft:` sin prefijo se dejan tal cual.
Todos los demás especificadores sin prefijo se les antepone la ruta relativa calculada hacia
`.miga_modules/`.

---

## Protocolo del registro

**`src/registry/mod.rs`**

La URL base del registro se lee desde `MIGA_REGISTRY_URL` (con fallback a un valor
predeterminado hardcodeado). Se usan dos endpoints HTTP:

| Endpoint                                | Propósito                                                         |
| --------------------------------------- | ----------------------------------------------------------------- |
| `GET /registry.json`                    | Manifiesto global del registro con la lista de todos los módulos. |
| `GET /modules/<nombre>/<nombre>.tar.gz` | Descarga del archivo del módulo.                                  |

**`src/registry/manifest.rs`** define `ProjectManifest` (`.miga/miga.json`)
y `ModuleManifest` (descriptor por módulo dentro de los archivos).

---

## Utilidades compartidas

| Módulo                                | Responsabilidades principales                                                                                                        |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `utils/project.rs`                    | `require_initialized`, `load_manifest`, `save_manifest`, `load_lock`, `save_lock` — fuente única de verdad para la E/S del proyecto. |
| `utils/net.rs`                        | `is_online()` — sonda TCP a `1.1.1.1:53` con un tiempo de espera de 1500 ms.                                                         |
| `utils/builder.rs`                    | `build_project(opts)` — orquesta compilación → copia a rutas de despliegue.                                                          |
| `utils/compiler/` → `compiler/mod.rs` | Pipeline TypeScript → JS basado en oxc.                                                                                              |
| `utils/env.rs`                        | Resuelve `DeployPaths` desde `.env` o valores predeterminados de la plataforma.                                                      |
| `utils/fs.rs`                         | `ensure_dir`, `clean_dir`, `copy_dir`, `exists`, `write_if_not_exists`.                                                              |
| `utils/json.rs`                       | `read_json`, `write_json`, `to_unicode_escapes`.                                                                                     |
| `utils/npm.rs`                        | Descarga archivos `.d.ts` desde el registro de npm.                                                                                  |
| `utils/output.rs`                     | Funciones auxiliares de salida en terminal coherentes (`section`, `step`, `success`, `error`, `warn`).                               |
| `utils/tsconfig.rs`                   | Escribe el `tsconfig.json` de mapeo de rutas TypeScript para la resolución de módulos.                                               |

---

## Estructura de archivos

```
src/
├── main.rs                 Punto de entrada
├── cli.rs                  Definiciones de comandos con clap
├── commands/
│   ├── mod.rs              Re-exportaciones
│   ├── init.rs
│   ├── add.rs
│   ├── fetch.rs
│   ├── run.rs
│   ├── build.rs
│   └── remove.rs
├── compiler/
│   └── mod.rs              Pipeline TypeScript → JS con oxc
├── registry/
│   ├── mod.rs              Cliente HTTP del registro
│   └── manifest.rs         Tipos ProjectManifest / ModuleManifest
└── utils/
    ├── mod.rs              Re-exportaciones de módulos
    ├── builder.rs          Orquesta compilación + despliegue
    ├── env.rs              Resolución de rutas de despliegue
    ├── fs.rs               Funciones auxiliares de sistema de archivos
    ├── json.rs             Funciones auxiliares de JSON
    ├── net.rs              Sonda de conectividad
    ├── npm.rs              Descargador de tipos npm
    ├── output.rs           Salida en terminal
    ├── project.rs          E/S del manifiesto del proyecto
    └── tsconfig.rs         Generador de tsconfig
```

---

## Estructuras de datos clave

### `ProjectManifest` (`.miga/miga.json`)

```json
{
    "name": "my-addon",
    "namespace": "woc",
    "version": "0.1.0",
    "modules": { "<nombre-del-módulo>": "<versión>" },
    "externals": { "@minecraft/server": "2.4.0" }
}
```

### `LockFile` (`.miga/modules.lock`)

```json
{
    "modules": {
        "<nombre-del-módulo>": {
            "version": "1.0.0",
            "files": ["behavior/scripts/...", "..."]
        }
    }
}
```

### `ModuleManifest` (dentro de cada archivo de módulo)

```json
{
    "name": "nombre-del-módulo",
    "version": "1.0.0",
    "description": "...",
    "dependencies": ["otro-módulo"]
}
```

### `CompileOptions`

```rust
pub struct CompileOptions {
    pub obfuscate: bool,
    pub source_maps: bool,
}
```
