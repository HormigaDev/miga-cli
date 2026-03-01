# miga

> **Gerenciador de Pacotes de Utilitários para Addons do Bedrock**
>
> Uma CLI rápida e sem dependências que inicializa, compila, empacota e gerencia
> add-ons do Minecraft Bedrock Edition — escrita em Rust.

[![Licença: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](../../LICENSE)

<div style="text-align: center">
<img src="../../assets/miga_banner.png" alt="Miga Banner" height="150">
</div>

---

## Índice

- [Visão Geral](#visão-geral)
- [Instalação](#instalação)
- [Comandos](#comandos)
    - [init](#init)
    - [add](#add)
    - [fetch](#fetch)
    - [run](#run)
    - [build](#build)
    - [remove](#remove)
- [Estrutura do projeto](#estrutura-do-projeto)
- [Variáveis de ambiente](#variáveis-de-ambiente)
- [Contribuindo](#contribuindo)
- [Licença](#licença)

---

## Visão Geral

`miga` substitui toda uma cadeia de ferramentas Node.js para o desenvolvimento de add-ons do Bedrock.
Ele cuida de:

- **Scaffolding** — cria um projeto BP + RP completo com suporte a TypeScript tipado.
- **Tipos TypeScript** — baixa arquivos `.d.ts` diretamente do registro do npm sem
  exigir que `npm` ou `node` estejam instalados.
- **Módulos do registro** — obtém módulos da comunidade a partir do registro do miga e os integra
  ao seu projeto.
- **Compilação** — transpila e, opcionalmente, minifica TypeScript usando
  [oxc](https://oxc.rs/) (Rust nativo, ~100× mais rápido que `tsc`).
- **Empacotamento** — monta arquivos `.mcpack` e `.mcaddon` prontos para distribuição.
- **Hot reload** — monitora os arquivos-fonte e os reimplanta nas pastas de packs de desenvolvimento
  do Minecraft a cada salvamento.

---

## Instalação

### A partir do código-fonte

```bash
git clone https://github.com/HormigaDev/miga.git
cd miga
cargo install --path .
```

### Binários pré-compilados

Baixe a versão mais recente na
[página de Releases](https://github.com/HormigaDev/miga-cli/releases) e coloque o binário
em algum lugar do seu `PATH`.

---

## Comandos

### `init`

Inicializa um novo projeto de add-on do Bedrock de forma interativa.

```bash
miga init [--namespace <ns>] [--name <n>]
```

**Opções**

| Flag          | Descrição                                                   |
| ------------- | ----------------------------------------------------------- |
| `--namespace` | Prefixo de namespace usado dentro do add-on (ex.: `woc`).   |
| `--name`      | Identificador interno do add-on (ex.: `ecological-spawns`). |

As opções ausentes são solicitadas interativamente. O comando cria um diretório com o
nome do add-on contendo um esqueleto completo de BP/RP com suporte a TypeScript.

---

### `add`

Adiciona um pacote de tipos `@minecraft/*` do registro do npm.

```bash
miga add <pacote[@versão]> [<pacote[@versão]> ...]
```

**Exemplos**

```bash
miga add @minecraft/server@2.4.0
miga add @minecraft/server @minecraft/common
```

Os tipos são baixados para `.miga_modules/` e o pacote é registrado em
`.miga/miga.json`.

---

### `fetch`

Instala um ou mais módulos a partir do **registro do miga**.

```bash
miga fetch <módulo> [<módulo> ...]
```

Os módulos são baixados, extraídos e registrados em `.miga/modules.lock`.
As dependências transitivas são resolvidas automaticamente.

---

### `run`

Monitora alterações no código-fonte e recarrega o add-on no Minecraft em tempo real.

```bash
miga run
```

`miga run` compila TypeScript a cada alteração e copia os packs para os caminhos
configurados em `.env` (`BEHAVIOR_PACKS_PATH` / `RESOURCE_PACKS_PATH`).

---

### `build`

Compila e empacota o add-on.

```bash
miga build
```

Saídas:

| Arquivo                 | Descrição                           |
| ----------------------- | ----------------------------------- |
| `dist/<nome>-bp.mcpack` | Apenas o Behavior Pack.             |
| `dist/<nome>-rp.mcpack` | Apenas o Resource Pack.             |
| `dist/<nome>.mcaddon`   | Arquivo combinado (ambos os packs). |

---

### `remove`

Remove um módulo instalado do registro.

```bash
miga remove <módulo>
```

Apaga os arquivos do módulo e o remove de `.miga/modules.lock`.

---

## Estrutura do projeto

Após executar `miga init`, o projeto terá a seguinte estrutura:

```
<nome-do-addon>/
├── behavior/               Behavior Pack
│   ├── manifest.json
│   ├── pack_icon.png       Substitua pelo seu próprio ícone
│   ├── LICENSE
│   └── scripts/
│       ├── index.ts        Ponto de entrada
│       ├── config/
│       │   └── registry.ts Registro central / namespace
│       ├── events/
│       │   └── index.ts
│       ├── components/
│       ├── features/
│       └── core/
├── resource/               Resource Pack
│   ├── manifest.json
│   ├── pack_icon.png
│   ├── LICENSE
│   ├── texts/              en_US.lang, es_ES.lang, pt_BR.lang
│   ├── textures/
│   │   ├── blocks/
│   │   ├── items/
│   │   ├── entity/
│   │   └── ui/
│   ├── models/
│   ├── sounds/
│   └── ui/
├── .miga/
│   ├── miga.json           Manifesto do projeto (nome, versão, módulos)
│   └── modules.lock        Arquivo de lock dos módulos instalados
├── .env                    Caminhos de implantação (não deve ser versionado)
├── .env.template           Modelo para compartilhar com colaboradores
├── .gitignore
├── tsconfig.json
├── LICENSE
└── README.md
```

---

## Variáveis de ambiente

Configure o `.env` (copie de `.env.template`):

```dotenv
# Caminho absoluto para a pasta development_behavior_packs do Minecraft
BEHAVIOR_PACKS_PATH=

# Caminho absoluto para a pasta development_resource_packs do Minecraft
RESOURCE_PACKS_PATH=

# true = source maps inline (apenas para depuração)
SOURCE_MAPS=false
```

No Linux, os caminhos padrão são detectados automaticamente via `$HOME`. No Windows, apontam para
`%LOCALAPPDATA%\Packages\Microsoft.MinecraftUWP_*`. Se o caminho não for encontrado,
o miga emitirá um aviso e ignorará a etapa de cópia.

---

## Contribuindo

Consulte [CONTRIBUTING.md](../../CONTRIBUTING.md).

---

## Licença

`miga` é software livre publicado sob a
[Licença Pública Geral GNU v3.0](../../LICENSE).
