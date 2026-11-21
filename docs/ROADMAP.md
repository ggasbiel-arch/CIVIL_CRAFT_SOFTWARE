# Roadmap incremental — Projeto 2

## E0 — Fundação técnica [INICIADA]
- [x] Monorepo/workspace Rust separado do Projeto 1.
- [x] Modelo tipado experimental: paredes, lajes, pavimentos.
- [x] Serialização JSON, validação, quantitativos brutos.
- [x] Editor 2D básico + prévia isométrica + CLI Python.
- [x] Testes de lógica Python e testes Rust incluídos no código.
- [ ] Compilar e testar aplicativo Rust em Windows x64 com Cargo.
- [ ] Testes funcionais manuais de desenho, seleção, abrir/salvar, undo/redo.

**Aceite E0:** aplicativo compila em máquina real, cria ≥4 paredes, salva, reabre sem perda de dados e passa `cargo test --workspace`.

## E0.2 — Componentes e interoperabilidade [PROTÓTIPO PARCIAL]
- [x] Especificação do objeto BIM universal e diretrizes de conversão SKP/RFA/GSM/IFC.
- [x] Definições e instâncias tipadas em Rust, origem de importação e propriedades BIM.
- [x] Avaliador de grafo numérico em Rust e ferramenta auxiliar Python com testes.
- [ ] Compilar módulo Rust e validar os testes no Windows.
- [x] Vincular definições e instâncias ao documento e representar a porta paramétrica na planta e na isometria.
- [ ] Integrar SDK SketchUp (adaptador .skp real) e começar IFC (adaptador real).

**Aceite E0.2:** abrir exemplo de porta, alterar valor de instância e obter saídas numéricas atualizadas sem alterar a definição; preservar metadados de origem. **Importação proprietária não está implementada.**

## E1 — Geometria CAD e editor
- [ ] Abstração de kernel com testes de sólidos.
- [ ] B-Rep integrado, extrusão, booleana, operações em faces.
- [ ] Viewport 3D GPU com seleção/picking por elemento e face.
- [ ] Snaps robustos, grade, cotas temporárias, gizmos e comandos.
- [ ] Modelo em camadas/seções e histórico transacional persistente.

**Aceite:** extrusão/recorte persistem e podem ser reeditados após salvar/abrir.

## E1.5 — Modelagem procedural e editor visual
- [ ] Editor de nós com conexões tipadas, canvas, preview e depuração.
- [ ] Grafo reativo incremental e validação dimensional/unidades.
- [ ] Associar outputs do grafo às operações geométricas e à geração de sólidos.
- [ ] Criar componente paramétrico a partir de geometria livre e restrições.
- [ ] Python como nó de script opcional, seguro, em processo isolado.

**Aceite:** componente modelo muda de geometria após mudança de parâmetro e preserva definição compartilhada entre instâncias.

## E2 — BIM arquitetônico
- [ ] Paredes compostas, lajes, pilares, vigas, coberturas.
- [ ] Aberturas que recortam paredes; portas/janelas associativas.
- [ ] Níveis, constraints, famílias paramétricas e propriedades BIM.
- [ ] Quantitativos líquidos com regras, IDs e classificação.

**Aceite:** modificar espessura/altura atualiza planta, 3D e quantitativo sem refazer o modelo.

## E2.5 — Importadores de objetos e famílias
- [ ] .SKP: SDK e objetos/atributos/dynamic components disponíveis.
- [ ] .RFA: conversor baseado na API oficial do Revit, sob dependências/licenças cabíveis.
- [ ] .GSM: converter e interpretar subconjunto compatível GDL via caminhos oficiais.
- [ ] Eberick: IFC com Psets estruturais e indicação clara de dados perdidos.
- [ ] Assistente 'Transformar em BIM' e relatório de fidelidade da conversão.

**Aceite:** modelos-teste reais de cada origem geram objetos com geometria e metadados disponíveis, preservando ou relatando a perda de parametrização.

## E3 — Documentação e interoperabilidade
- [ ] Vistas associativas, plantas/cortes/elevações, impressão em escala.
- [ ] Símbolos, cotas e estilos com configurações para normas aplicáveis.
- [ ] IFC4/IFC4.3 import/export com round-trip e verificação semântica.
- [ ] DXF export para documentação 2D.

**Aceite:** modificação em uma parede repercute em plantas, cortes e quantitativos.

## E4 — Engenharia estrutural (fluxo Eberick)
- [ ] Modelo analítico separado do geométrico.
- [ ] Cargas/combinações, FEM, estabilidade e relatórios transparentes.
- [ ] Normas brasileiras aplicáveis mapeadas e parametrizadas por versão.
- [ ] Dimensionamento e detalhamento sob validação de responsável técnico.
- [ ] Suite de benchmarks, tolerâncias e regressão numérica.

**Aceite:** resultados conferem com casos independentes reconhecidos, com rastreabilidade e revisão profissional. Não tratar como ferramenta apta a projetos estruturais reais antes desse marco.

## E5 — Fluxos profissionais e colaboração
- [ ] Biblioteca de famílias e materiais; organização de projeto.
- [ ] Controle de revisões, worksharing, conflict resolution e autosave.
- [ ] API Python com políticas de extensão, pacotes, documentação.
- [ ] Testes de desempenho, QA, installers e hardening.

## Prioridade imediata

Validar E0/E0.2 no Windows, corrigir problemas detectados, então iniciar E1 (kernel CAD, viewport 3D e seleção de faces). **Não iniciar FEM estrutural antes de estabilizar núcleo geométrico e semântico.**
