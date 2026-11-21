# PROJETO 2 — CAD/BIM independente

**Status:** protótipo técnico 0.2, preparado para implementação Codex (pacote de publicação 0.3; etapa E0.2 parcial). Este repositório **não é** o Projeto 1 (plugin SketchUp).

## Objetivo

Construir uma plataforma CAD/BIM independente para arquitetura, modelagem direta, documentação e, em uma fase especializada e validada, engenharia estrutural. Inspirada funcionalmente em Archicad, SketchUp, Revit e Eberick, mas sem copiar código-fonte, arquivos proprietários, identidade visual ou marcas.

## Requisitos mandatórios da versão planejada

- Importar componentes do SketchUp (.skp), transformando-os em componentes BIM.
- Importar famílias do Revit (.rfa), objetos do Archicad (.gsm) e dados estruturais Eberick/IFC, mantendo parâmetros e propriedades **quando tecnicamente acessíveis** e apresentando diagnóstico de perdas de fidelidade.
- Modelagem **livre**, **paramétrica**, dinâmica por definição/instância e **visual por nós** (inspirada funcionalmente no Grasshopper/Blueprint).
- Rust 75% / Python 25% como **metas arquiteturais**, não proporção garantida no executável.

Consulte `docs/INTEROPERABILIDADE_E_PARAMETRIZACAO.md` para detalhes e limitações.

## Entregue neste starter

- Núcleo BIM com esquema experimental e tipado para pavimentos, paredes e lajes.
- Validação de IDs, dimensões e referências; cálculo preliminar de quantitativos **brutos**.
- Editor desktop com planta 2D, grade, snap, desenho de paredes, seleção, exclusão, desfazer/refazer.
- Vista **isométrica ilustrativa** baseada nos elementos, sem renderizador 3D real nem B-Rep.
- Abrir/salvar arquivos `.bim.json` (formato experimental, próprio do protótipo).
- Ferramenta Python para validar, listar quantitativos e inserir paredes no arquivo.
- Modelo demonstrativo e testes básicos.
- **Novo E0.2:** tipos Rust para definições/instâncias BIM e origem de importação, com grafo paramétrico **numérico** e testes Rust; ferramenta auxiliar Python para avaliar parâmetros.
- Exemplo de porta com definição reutilizável e override por instância (`samples/porta_parametrica.component.json`).

**Não entregue:** importadores reais SKP/RFA/GSM/Eberick, motor CAD B-Rep, editor gráfico por nós, geração geométrica a partir do grafo, integração de componentes à viewport, desenho de lajes na interface, portas/janelas BIM nativas, cortes/elevações, IFC, RVT/PLN, cálculos estruturais, norma ABNT automatizada, LOD, BIM colaborativo e orçamento executivo.

## Pré-requisitos (Windows 64 bits recomendado)

1. Instale Rust via https://rustup.rs e selecione a toolchain stable MSVC. A instalação pode exigir Microsoft C++ Build Tools.
2. Instale Python 3.11+ (opcional: apenas para scripts e testes Python).
3. Na pasta raiz, abra o PowerShell.

## Executar o programa

```powershell
cargo run --release -p civil_desktop
```

O Cargo baixa dependências da internet na primeira compilação. A janela exibe um modelo de exemplo. Escolha **Parede** e clique duas vezes na planta para indicar início e fim de uma parede. Com **Selecionar**, clique numa parede para selecioná-la ou use a lista à esquerda. Para salvar/abrir, informe o caminho na barra inferior e use os botões superiores.

**Observação:** esta versão inicial não foi compilada no ambiente de criação do pacote, que não dispõe de Rust/Cargo. Execute `cargo test --workspace` no seu PC para verificar integração com suas versões e dependências.

## Comandos de automação Python

```powershell
python python/civil_tools.py validate samples/casa_piloto.bim.json
python python/civil_tools.py summary samples/casa_piloto.bim.json
python python/civil_tools.py add-wall samples/casa_piloto.bim.json saida.bim.json --start 0 0 --end 3 0
python python/component_tools.py evaluate samples/porta_parametrica.component.json
python python/component_tools.py set samples/porta_parametrica.component.json porta_maior.json largura_m 1.20
python -m unittest discover -s tests -v
```

A ferramenta Python usa apenas a biblioteca padrão.

## Arquivos

```text
Cargo.toml                       # Workspace Rust
crates/civil_core/src/lib.rs     # Modelo BIM/validação/quantitativos
crates/civil_core/src/components.rs # Definições/instâncias e grafo numérico
crates/civil_desktop/src/main.rs # Interface desktop e canvas experimental
python/civil_tools.py           # Primeira API de automação em Python
python/component_tools.py       # Avaliador auxiliar de componentes paramétricos
samples/casa_piloto.bim.json    # Exemplo
samples/porta_parametrica.component.json # Componente dinâmico experimental
tests/test_civil_tools.py       # Testes de automação Python
tests/test_component_tools.py    # Testes do grafo paramétrico
docs/ARQUITETURA.md             # Fronteiras dos subsistemas
docs/INTEROPERABILIDADE_E_PARAMETRIZACAO.md # Requisitos e adaptadores
docs/ROADMAP.md                 # Entregas e critérios
```

## Política de engenharia

Quantitativos são **brutos**: não deduzem encontros de paredes ou vazios. Não podem ser usados como orçamento executivo sem revisão. Nenhum cálculo estrutural ou verificação de normas de segurança está implementado.

## Licenciamento

A licença do projeto ainda deve ser escolhida. Bibliotecas integradas no futuro terão suas respectivas licenças e obrigações verificadas; não há material proprietário de softwares concorrentes neste protótipo.

## GitHub e início no Codex

- Para publicar este Projeto 2 em um repositório **novo e separado**, consulte `PUBLICAR_NO_GITHUB.md` ou execute `PUBLICAR_NO_GITHUB.ps1` com Git + GitHub CLI autenticados.
- O prompt completo da **primeira tarefa de implementação** está em `CODEX_START_PROMPT.md`.
- As regras permanentes do agente estão em `AGENTS.md`.
- A presença desses arquivos não significa que importadores externos ou kernel CAD já estejam implementados.
