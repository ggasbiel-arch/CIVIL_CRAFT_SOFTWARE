# CIVIL CRAFT CAD/BIM — Projeto 2

Software CAD/BIM independente, separado do Projeto 1 (plugin SketchUp).

## E2 — Documentação e PDF vetorial (incremento)

A etapa E2 implementa o vínculo de vistas ao modelo existente:
- Plantas de pavimento derivadas dos contornos armazenados; cortes por interseção geométrica; elevações por projeção vetorial de arestas.
- A viewport mostra a geometria read-only dos elementos já gravados, sem reintroduzir as ferramentas provisórias de parede/janela/laje.
- Criação de pranchas A3, posicionamento automático inicial de vistas salvas e pré-visualização.
- **Prancha PDF vetorial** exportável diretamente ou por Conjuntos Publicadores, inclusive wireframe de vistas 3D salvas.

**Atenção:** não há remoção de linhas ocultas nem escala de impressão real 1:N. Os desenhos usam ajuste ao quadro e não devem ser usados como documentos executivos. Projetos vazios continuam sem geometria. Veja [E2 — documentação preliminar](docs/DOCUMENTACAO_PRELIMINAR_E2.md).

## E1 — Câmeras, cenas e Navigator

A etapa E1 adiciona ao código atual: câmeras ortogonais, perspectiva, perspectiva de dois pontos, órbita e pan; seleção por janela/cruzamento de marcadores, mapas de pavimentos/pontos de vista, vistas salvas, tabelas, conjuntos publicadores e informações do projeto.

- **Arquivo → Informações do Projeto**: cliente, local, arquiteto, fase, revisão, código e campos personalizados.
- **Mapa do Projeto**: pavimentos e viewpoints 3D, plantas, cortes, elevações e detalhes.
- **Vistas/Tabelas**: snapshots de câmera, opções de escala/camadas, tabelas de pavimentos, elementos e vistas.
- **Publicador**: listas vinculadas, exportação inicial JSON de vistas e CSV de tabelas.
- **Controles 3D**: Shift+meio ou direito arrastado = órbita; meio arrastado = pan; roda = zoom.
- **Seleção**: esquerda→direita = contém; direita→esquerda = cruza, aplicada a marcadores de viewpoint.

**Importante:** o visualizador ainda desenha uma grade 3D e marcadores, não sólidos BIM. Cortes e elevações são registros no navegador; geração documental e PDF/IFC virão depois. As ferramentas de paredes, pisos/lajes e janelas de teste permanecem removidas.

Documentação detalhada: [Câmeras, cenas e publicador](docs/CAMERAS_CENAS_E_PUBLICADOR.md).

## Limpeza anterior da base

As ferramentas provisórias de teste de **parede, janela e piso/laje** foram removidas. O starter possuía desenho de paredes por clique, prévias ilustrativas de paredes e lajes, quantitativos brutos demonstrativos e a casa piloto. Esses recursos não constituíam ferramentas BIM definitivas. Não havia uma ferramenta real de janela nessa base.

O aplicativo inicia com **um projeto vazio**. Oferece uma viewport de grade projetada, câmeras orbitáveis, um navegador com três mapas e salvamento de cenas. A geometria BIM definitiva ainda não está integrada.

Preservamos no núcleo Rust o schema experimental com tipos legados de parede/laje para **abrir arquivos antigos sem descartar seus dados**, mas sem oferecer ferramentas de criação ou renderização desses elementos. O grafo numérico paramétrico e os utilitários Python de validação também permanecem disponíveis.

**Ainda não implementado:** kernel CAD B-Rep e geometria BIM avançada, editor visual por nós, importadores SKP/RFA/GSM/IFC, ferramentas BIM profissionais de paredes/pisos/janelas, documentação executiva completa com escalas físicas e análise estrutural validada.

## Objetivo do produto

Construir uma plataforma própria de modelagem livre e paramétrica, objetos BIM responsivos, programação visual por nós e interoperabilidade com SketchUp, Revit, Archicad e Eberick, respeitando limitações de formatos e licenças. Meta de arquitetura: Rust 75% e Python 25%.

## Executar e testar

Instale Rust stable e, opcionalmente, Python 3.11+.

```powershell
cargo run --release -p civil_desktop
cargo test --workspace
python -m unittest discover -s tests -v
python python/civil_tools.py validate samples/projeto_vazio.bim.json
python python/civil_tools.py summary samples/projeto_vazio.bim.json
```

Os testes Python foram executados durante a limpeza. A compilação Rust ainda requer validação num ambiente com Cargo.

## Organização

- `crates/civil_core/src/lib.rs` — schema legado e validação.
- `crates/civil_core/src/components.rs` — definições/instâncias BIM e grafo numérico.
- `crates/civil_desktop/src/main.rs` — shell desktop neutro.
- `python/civil_tools.py` — validação e inspeção, sem `add-wall`.
- `samples/projeto_vazio.bim.json` — projeto inicial vazio.
- `samples/porta_parametrica.component.json` — teste numérico isolado de componentes (não é ferramenta de porta/janela).
- `AGENTS.md` e `docs/` — arquitetura e roadmap.

Novas ferramentas complexas devem ser implementadas com geometria e comportamento verificáveis, sem ressuscitar previews ou ferramentas fictícias usadas nos primeiros testes.
