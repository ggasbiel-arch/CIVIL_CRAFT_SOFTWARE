# PROJETO 2 — CAD/BIM independente

**Status:** protótipo técnico 0.2, preparado para implementação Codex (pacote de publicação 0.3; etapa E0.2 com porta paramétrica nativa). Este repositório **não é** o Projeto 1 (plugin SketchUp).

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
- Abrir e salvar projetos `.CCW` com o esquema BIM versionado em JSON. Arquivos anteriores `.bim.json` também podem ser abertos. Ao salvá-los, o editor cria uma cópia `.CCW`.
- Ferramenta Python para validar, listar quantitativos e inserir paredes no arquivo.
- Modelo demonstrativo e testes básicos.
- **Novo E0.2:** tipos Rust para definições/instâncias BIM e origem de importação, com grafo paramétrico **numérico** e testes Rust; ferramenta auxiliar Python para avaliar parâmetros.
- Exemplo de porta com definição reutilizável e override por instância (`samples/porta_parametrica.component.json`).

**Não entregue:** importadores reais SKP/RFA/GSM/Eberick, motor CAD B-Rep, editor gráfico por nós, geração de sólidos a partir do grafo, recorte de paredes por portas, desenho de lajes na interface, janelas BIM nativas, cortes/elevações, IFC, RVT/PLN, cálculos estruturais, norma ABNT automatizada, LOD, BIM colaborativo e orçamento executivo.

## Pré-requisitos (Windows 64 bits recomendado)

1. Instale Rust via https://rustup.rs e selecione a toolchain stable MSVC. A instalação pode exigir Microsoft C++ Build Tools.
2. Instale Python 3.11+ (opcional: apenas para scripts e testes Python).
3. Na pasta raiz, abra o PowerShell.

## Executar o programa

```powershell
cargo run --release -p civil_desktop
```

O Cargo baixa dependências da internet na primeira compilação. A janela abre a Casa piloto em **Planta**, com a ferramenta **Selecionar** ativa.

- No menu **Arquivo**, use **Novo**, **Exemplo**, **Abrir** e **Salvar**. Informe o caminho `.CCW` no campo ao lado dos comandos de arquivo.
- A faixa **Modelagem** alterna entre **Paramétrica + visual por nós** e **Livre** no mesmo documento. O primeiro modo reúne as ferramentas de arquitetura e permite consultar as saídas do grafo do componente selecionado. O segundo permite navegar e selecionar. A edição visual dos nós e as ferramentas de geometria livre seguem planejadas.
- Na paleta **Ferramentas** à esquerda, escolha **Parede** e marque o início e o fim na planta. Em **Propriedades**, o grupo **Próxima parede** define as dimensões para o próximo desenho.
- O **Navegador BIM** lista pavimentos, elementos e quantitativos brutos. **Propriedades** mostra os dados da seleção e permite alterar a largura da porta. Use os controles do menu **Janelas** para ocultar ou reabrir cada painel.
- No menu **Câmeras**, alterne entre **Planta**, **Isometria** e **Duas vistas** e ajuste o zoom. O snap fica em **Ferramentas**. **Duas vistas** distribui a planta e a isometria na horizontal quando há espaço e na vertical quando o centro está estreito. Ao escolher uma ferramenta na isometria, a interface volta à planta.

### Porta paramétrica

1. Escolha **Porta** e clique na planta para posicionar a extremidade inicial no pavimento ativo. A porta é inserida ao longo do eixo X e selecionada automaticamente.
2. Para voltar à edição, clique na porta com **Selecionar** ou escolha sua instância no **Navegador BIM**.
3. Informe uma largura de 0,60 a 2,00 m no inspetor e clique em **Aplicar largura**. A planta mostra o vão nominal. A vista isométrica mostra a folha calculada pelo grafo. Com 1,20 m de largura, a folha tem 1,16 m.
4. Use **Desfazer** e **Refazer** para percorrer as edições da sessão. Salve e reabra o `.CCW` para recuperar as instâncias e seus parâmetros.

A porta é independente da parede. A representação é simbólica na planta e formada por um contorno na isometria. O histórico existe em memória durante a sessão. Consulte [o ADR da integração](docs/adr/0001-componentes-no-documento.md) para detalhes de persistência e validação.

## Comandos de automação Python

```powershell
python python/civil_tools.py validate samples/casa_piloto.bim.json
python python/civil_tools.py summary samples/casa_piloto.bim.json
python python/civil_tools.py add-wall samples/casa_piloto.bim.json saida.CCW --start 0 0 --end 3 0
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

### Consultar um arquivo IFC

No Windows x64, use Python 3.12 de 64 bits. A partir da pasta do repositório, prepare o ambiente e inicie o aplicativo na mesma sessão do PowerShell.

```powershell
py -3.12 -m venv .venv-ifc
.\.venv-ifc\Scripts\python.exe -m pip install -r python/requirements-ifc.txt
$env:CIVIL_CRAFT_PYTHON = (Resolve-Path .\.venv-ifc\Scripts\python.exe).Path
cargo run -p civil_desktop
```

Em **Arquivo → Consultar IFC**, informe o caminho do arquivo STEP com extensão `.ifc` e clique em **Ler elementos**. O IfcOpenShell identifica o esquema, conta todas as instâncias de IfcElement por classe e exibe nome e GlobalId das primeiras 100. A consulta funciona em uma janela de leitura. Conversão em elementos editáveis, geometria e exportação ficam para outras etapas.

### Biblioteca de normas

Abra **Ajuda → Biblioteca de normas ABNT** para pesquisar por código, assunto ou edição. A seleção inicial contém oito referências de construção civil, com descrições temáticas. Consulte o catálogo oficial pelo link da janela para verificar título, edição, vigência e disponibilidade do documento.

Em **Carregar catálogo local**, abra um índice TSV em UTF-8 com as colunas `codigo`, `assunto` e `edicao`, como em `biblioteca/referencias-abnt.tsv`. O índice pode abranger qualquer área e conter várias edições da mesma norma. Uma edição desconhecida pode ficar vazia. O catálogo carregado substitui a seleção exibida na sessão. Para usá-lo em cada inicialização, defina `CIVIL_CRAFT_ABNT_TSV` com o caminho do arquivo. A distribuição dos textos integrais e a sincronização com acervos comerciais dependem do fornecimento de uma fonte licenciada.

### Normas Regulamentadoras do trabalho

Em **Ajuda → Normas Regulamentadoras (NR)**, pesquise pelo número ou por palavras do título. O índice local consultado em 23/11/2026 contém NR-1 a NR-38 e inclui as revogadas NR-2 e NR-27. Use **Somente vigentes** para filtrar. Cada item abre a página correspondente do Ministério do Trabalho e Emprego no navegador. A consulta ao índice funciona offline e a leitura das publicações oficiais requer internet.
