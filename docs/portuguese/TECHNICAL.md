# miga — Referência Técnica

Este documento descreve a arquitetura interna do `miga` para contribuidores
e mantenedores.

---

## Índice

- [Arquitetura geral](#arquitetura-geral)
- [Ponto de entrada do binário](#ponto-de-entrada-do-binário)
- [Camada CLI](#camada-cli)
- [Comandos](#comandos)
- [Pipeline do compilador](#pipeline-do-compilador)
- [Protocolo do registro](#protocolo-do-registro)
- [Utilitários compartilhados](#utilitários-compartilhados)
- [Estrutura de arquivos](#estrutura-de-arquivos)
- [Estruturas de dados principais](#estruturas-de-dados-principais)

---

## Arquitetura geral

```
┌─────────────────────────────────────────────────┐
│                  binário miga                   │
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
│         (pipeline oxc)   (manifesto + HTTP)     │
│               │               │                 │
│          utils/ ─────────────────────────────── │
│          fs · json · npm · env · builder ·      │
│          output · tsconfig · net · project      │
└─────────────────────────────────────────────────┘
```

Todos os comandos retornam `anyhow::Result<()>`. Os erros se propagam até `main.rs`, onde
uma única chamada a `output::error()` os formata e encerra o processo.

---

## Ponto de entrada do binário

**`src/main.rs`**

1. Faz o parse da CLI com `Cli::parse()` (derive do clap).
2. Faz match sobre `Commands` e delega ao `commands::*::run()` correspondente.
3. Em caso de erro: imprime uma mensagem formatada via `utils::output::error()` e encerra
   com código 1.

---

## Camada CLI

**`src/cli.rs`**

Define `Cli` e `Commands` usando os macros derive do clap.

| Subcomando | Variante do struct                   |
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

1. Coleta metadados de forma interativa via `dialoguer::Input`.
2. Valida o namespace (minúsculas, sem espaços ou dois-pontos) e o nome.
3. Cria o diretório do projeto e o define como diretório de trabalho.
4. Gera UUIDs novos para todas as entradas do manifesto.
5. Escreve as árvores de diretórios de BP e RP, manifestos, scripts de ponto de entrada,
   arquivos de licença e modelos estáticos.
6. Escreve `.miga/miga.json` (manifesto do projeto) e `.miga/modules.lock`.
7. Chama `commands::add::run()` para instalar a versão escolhida de `@minecraft/server`.

O **conteúdo estático** é definido como strings `const` no final do arquivo.
O **conteúdo dinâmico** (manifestos, licenças, README) é gerado por funções auxiliares
dedicadas que recebem apenas o que necessitam.

---

### `add` (`src/commands/add.rs`)

Baixa pacotes de tipos TypeScript do registro do npm.

1. `utils::net::is_online()` — verificação de conectividade.
2. `utils::project::require_initialized()` — confirma que `.miga/` existe.
3. Para cada especificação de pacote: chama `utils::npm::fetch_types()`.
4. Registra `nome → versão` em `.miga/miga.json` sob `externals`.
5. Atualiza o bloco de dependências de `behavior/manifest.json` via
   `sync_behavior_manifest()`.

---

### `fetch` (`src/commands/fetch.rs`)

Instala módulos do registro do miga.

1. Verificação de conectividade.
2. `utils::project::require_initialized()`.
3. Para cada nome de módulo: resolve a árvore completa de dependências via
   `registry::resolve_dependencies()`.
4. Baixa o arquivo `.tar.gz` e o extrai.
5. Registra o módulo em `.miga/modules.lock`.

---

### `run` (`src/commands/run.rs`)

Loop de monitoramento com hot-reload.

1. `utils::builder::build_project()` — compilação e implantação inicial.
2. `notify::RecommendedWatcher` monitora `behavior/scripts/`.
3. A cada evento de alteração: re-executa `build_project()`.

---

### `build` (`src/commands/build.rs`)

Compilação completa e empacotamento.

1. `utils::builder::build_project()` — compilação TypeScript → JS + implantação.
2. Sincroniza a versão de `.miga/miga.json` em ambos os manifestos.
3. `dist/` é limpo, depois `zip` empacota o BP e o RP em arquivos `.mcpack`
   e os combina em um `.mcaddon`.

---

### `remove` (`src/commands/remove.rs`)

Remove um módulo do registro.

1. `utils::project::require_initialized()`.
2. Lê o arquivo de lock; verifica se o módulo está instalado.
3. Remove os arquivos do módulo do disco (`behavior/scripts/` e
   `.miga_modules/`).
4. Remove a aresta de dependência de `behavior/manifest.json`.
5. Escreve o arquivo de lock atualizado.

---

## Pipeline do compilador

**`src/compiler/mod.rs`**

Usa a família de crates do [oxc](https://oxc.rs/) para compilação de TypeScript sem Node.js.

```
fonte .ts
    │
    ▼
oxc_parser::Parser::parse()          — produz a AST
    │
    ▼
oxc_semantic::SemanticBuilder        — análise de escopo e bindings, produz
                                       o Scoping passado ao Transformer
    │
    ▼
oxc_transformer::Transformer         — TypeScript → JS ES2020
    │
    ▼
(opcional) oxc_minifier::Minifier    — eliminação de código morto + mangling
    │
    ▼
oxc_codegen::Codegen                 — emite a string JS final
```

**Reescrita de imports**: imports relativos com `./` e `../` têm sua extensão `.ts`
substituída por `.js`. Especificadores `minecraft:` sem prefixo são mantidos como estão.
Todos os outros especificadores sem prefixo recebem o caminho relativo calculado para
`.miga_modules/` como prefixo.

---

## Protocolo do registro

**`src/registry/mod.rs`**

A URL base do registro é lida de `MIGA_REGISTRY_URL` (com fallback para um valor padrão
hardcoded). Dois endpoints HTTP são utilizados:

| Endpoint                            | Propósito                                               |
| ----------------------------------- | ------------------------------------------------------- |
| `GET /registry.json`                | Manifesto global do registro listando todos os módulos. |
| `GET /modules/<nome>/<nome>.tar.gz` | Download do arquivo do módulo.                          |

**`src/registry/manifest.rs`** define `ProjectManifest` (`.miga/miga.json`)
e `ModuleManifest` (descritor por módulo dentro dos arquivos).

---

## Utilitários compartilhados

| Módulo                                | Responsabilidades principais                                                                                                    |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `utils/project.rs`                    | `require_initialized`, `load_manifest`, `save_manifest`, `load_lock`, `save_lock` — fonte única de verdade para E/S do projeto. |
| `utils/net.rs`                        | `is_online()` — sonda TCP para `1.1.1.1:53` com timeout de 1500 ms.                                                             |
| `utils/builder.rs`                    | `build_project(opts)` — orquestra compilação → cópia para caminhos de implantação.                                              |
| `utils/compiler/` → `compiler/mod.rs` | Pipeline TypeScript → JS baseado em oxc.                                                                                        |
| `utils/env.rs`                        | Resolve `DeployPaths` a partir de `.env` ou padrões da plataforma.                                                              |
| `utils/fs.rs`                         | `ensure_dir`, `clean_dir`, `copy_dir`, `exists`, `write_if_not_exists`.                                                         |
| `utils/json.rs`                       | `read_json`, `write_json`, `to_unicode_escapes`.                                                                                |
| `utils/npm.rs`                        | Baixa arquivos `.d.ts` do registro do npm.                                                                                      |
| `utils/output.rs`                     | Funções auxiliares de saída no terminal consistentes (`section`, `step`, `success`, `error`, `warn`).                           |
| `utils/tsconfig.rs`                   | Escreve o `tsconfig.json` de mapeamento de caminhos TypeScript para resolução de módulos.                                       |

---

## Estrutura de arquivos

```
src/
├── main.rs                 Ponto de entrada
├── cli.rs                  Definições de comandos com clap
├── commands/
│   ├── mod.rs              Re-exportações
│   ├── init.rs
│   ├── add.rs
│   ├── fetch.rs
│   ├── run.rs
│   ├── build.rs
│   └── remove.rs
├── compiler/
│   └── mod.rs              Pipeline TypeScript → JS com oxc
├── registry/
│   ├── mod.rs              Cliente HTTP do registro
│   └── manifest.rs         Tipos ProjectManifest / ModuleManifest
└── utils/
    ├── mod.rs              Re-exportações de módulos
    ├── builder.rs          Orquestra compilação + implantação
    ├── env.rs              Resolução de caminhos de implantação
    ├── fs.rs               Funções auxiliares de sistema de arquivos
    ├── json.rs             Funções auxiliares de JSON
    ├── net.rs              Sonda de conectividade
    ├── npm.rs              Baixador de tipos npm
    ├── output.rs           Saída no terminal
    ├── project.rs          E/S do manifesto do projeto
    └── tsconfig.rs         Gerador de tsconfig
```

---

## Estruturas de dados principais

### `ProjectManifest` (`.miga/miga.json`)

```json
{
    "name": "my-addon",
    "namespace": "woc",
    "version": "0.1.0",
    "modules": { "<nome-do-módulo>": "<versão>" },
    "externals": { "@minecraft/server": "2.4.0" }
}
```

### `LockFile` (`.miga/modules.lock`)

```json
{
    "modules": {
        "<nome-do-módulo>": {
            "version": "1.0.0",
            "files": ["behavior/scripts/...", "..."]
        }
    }
}
```

### `ModuleManifest` (dentro de cada arquivo de módulo)

```json
{
    "name": "nome-do-módulo",
    "version": "1.0.0",
    "description": "...",
    "dependencies": ["outro-módulo"]
}
```

### `CompileOptions`

```rust
pub struct CompileOptions {
    pub obfuscate: bool,
    pub source_maps: bool,
}
```
