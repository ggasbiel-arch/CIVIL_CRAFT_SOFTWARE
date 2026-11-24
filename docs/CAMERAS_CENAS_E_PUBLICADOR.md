# E1 — Câmeras, cenas, mapas e publicador

## Estado do desenvolvimento

Implementado no núcleo Rust em `crates/civil_core/src/navigation.rs`:
- `Camera`: ortogonal, perspectiva de 1 quadro e perspectiva de dois pontos, com conversão de coordenadas 3D para pixels, órbita, pan, zoom e validação.
- A perspectiva de dois pontos usa orientação horizontal e eixo vertical fixo, sem convergência vertical.
- Seleção espacial por janela: esquerda → direita exige inclusão completa; direita → esquerda exige cruzamento.
- `Workspace`: pontos de vista (planta, corte, elevação, detalhe e 3D), vistas salvas, tabelas e conjuntos publicadores.
- `ProjectInformation`: código, endereço, cliente, responsável, engenharia, fase, descrição, revisão e propriedades customizadas.
- Serialização JSON de todos esses dados com `serde`, preservando a leitura dos projetos v1 anteriores com `#[serde(default)]`.
- Testes de câmera, seleção, persistência e integridade referencial de publicadores.

Implementado na interface desktop em `crates/civil_desktop/src/main.rs`:
- Viewport geométrica real em 2D projetado: grade métrica, eixos XYZ e marcadores de viewpoints, atualizados conforme a câmera.
- Barra de projeção 3D: Ortogonal / Perspectiva / Dois pontos e Enquadrar.
- Órbita com Shift + botão do meio ou arrastar com o botão direito; pan com botão do meio; zoom na roda.
- Seleção com botão esquerdo em dois sentidos. É aplicada aos marcadores de viewpoints; NÃO há sólidos de paredes/lajes.
- Botão Orbitar ou tecla O habilita arrastar com o botão esquerdo para órbita (nesse modo, desabilita seleção por arrasto).
- Mapa do Projeto: criação de pavimentos e viewpoints de plantas, cortes, elevações, detalhes e 3D.
- Aba Vistas/Tabelas: cria vistas por snapshot de câmera, restaura, edita escala, camadas e observações; cria tabelas de pavimentos, elementos e vistas.
- Aba Publicador: conjuntos com referências a vistas e tabelas; exporta vistas como JSON e tabelas reais do projeto como CSV em uma pasta.
- Menu Arquivo → Informações do Projeto: formulário e campos personalizados.
- Comandos Novo, Abrir, Salvar, incluindo informações de projeto, câmeras, vistas e conjuntos.

## Convenções de entrada

| Interação na viewport | Comportamento |
|---|---|
| Shift + botão do meio + arrastar | Orbitar |
| Botão direito + arrastar | Orbitar |
| Botão do meio + arrastar | Pan |
| Roda do mouse | Zoom |
| Botão esquerdo esquerda→direita | Selecionar apenas marcadores inteiramente contidos (janela) |
| Botão esquerdo direita→esquerda | Selecionar todos os marcadores cruzados (interseção) |
| Orbitar [O] + botão esquerdo | Orbitar temporariamente em vez de selecionar |

## Escopo deliberadamente pendente

A grade/eixos são um visualizador geométrico de coordenadas, **não um renderizador BIM de sólidos**. Os marcadores de cortes, elevações e detalhes ainda não geram linhas de corte nem pranchas: são entidades persistidas no mapa. Tabelas contam e listam dados realmente armazenados, não quantitativos estruturais calculados. O publicador exporta JSON/CSV; publicação em PDF, IFC, DWG, imagens e folhas depende do futuro motor documental.

Próximas entregas: viewport GPU com renderização de entidades CAD/BIM, ferramentas de anotação/posicionamento de cortes, pranchas/layouts, agrupamento por pastas, estilos gráficos/layers e exportação PDF validada.

Não reintroduzir ferramentas temporárias de parede, janela ou piso/laje removidas a pedido do responsável.
