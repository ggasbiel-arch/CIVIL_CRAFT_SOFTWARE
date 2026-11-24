# PROMPT PARA O CODEX — PRIMEIRA IMPLEMENTAÇÃO DO PROJETO 2

Você é o engenheiro principal de um novo software CAD/BIM desktop independente chamado **CIVIL CRAFT CAD/BIM**. Este é o **PROJETO 2** e **não** é o plugin Ataliba BIM para SketchUp (Projeto 1). Não altere o repositório CIVIL_CRAFT_WORK.

## Missão
Abra o repositório e **comece a implementar código executável**, aproveitando o starter existente. Não entregue apenas arquitetura, protótipos em imagem ou plano teórico. Priorize construir um produto real, testado e incremental, com primeiro alvo **Windows x64**.

## Objetivo de longo prazo (não alegar pronto)
Software original que reúna:
1. edição geométrica livre estilo CAD/SketchUp, com sólidos e modelagem direta;
2. objetos BIM semânticos, definidos por tipos e instâncias com relações, propriedades e restrições paramétricas semelhantes em capacidade às oferecidas por Archicad/Revit;
3. modelagem visual por grafos de nós inspirada funcionalmente em Grasshopper/Blueprint; construção de componentes dinâmicos sem programação obrigatória;
4. importação SKP com atribuição BIM e importação fiel **quando tecnicamente possível** de RFA, GSM/GDL e modelos estruturais Eberick via IFC, com relatório de perdas e metadados de proveniência;
5. documentação, IFC e, futuramente, ferramentas estruturais validadas em subsistema separado.

Meta arquitetural: cerca de **75% Rust / 25% Python**, não adulterar arquitetura apenas para atingir uma contagem exata. Rust domina engine, UI, BIM, grafos e persistência; Python dá scripting, ferramentas, plugins e testes. Não usar código, assets, algoritmos proprietários, bibliotecas não autorizadas, marcas ou interfaces idênticas dos programas citados.

## Contexto do starter
Leia obrigatoriamente `AGENTS.md`, `README.md`, `docs/ARQUITETURA.md`, `docs/ROADMAP.md` e `docs/INTEROPERABILIDADE_E_PARAMETRIZACAO.md`. Inspecione o workspace Cargo e os testes. Há `civil_core` (schema legado e grafo numérico em `components.rs`), `civil_desktop` (interface neutra 2D/3D), scripts Python e amostras. As ferramentas demonstrativas de parede/piso foram removidas; não as restaure. **O Rust nunca foi compilado no ambiente que gerou o starter**; teste antes de concluir.

## Sprint 1 — IMPLEMENTAR AGORA

### A. Confiabilidade da base
- Execute `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` e `python -m unittest discover -s tests -v`.
- Corrija os erros de compilação, warnings relevantes e falhas reais sem remover os testes. Se precisar mudar APIs de eframe/egui, documente.
- Documente como executar o aplicativo no Windows e registre explicitamente o que foi de fato validado.

### B. Modelo BIM integrado ao documento
- Unifique os tipos `ComponentDefinition`, `ComponentInstance` e `ParametricGraph` do `civil_core::components` com `Project`, mantendo leitura compatível dos `.bim.json` anteriores por meio de defaults/migração controlada.
- Acrescente campos de versão do esquema e referências estáveis. As instâncias precisam referenciar definições; IDs duplicados e refs quebradas geram erros, nunca perda silenciosa.
- Armazene classificação BIM, propriedades editáveis, unidades, material opcional e metadados de origem/proveniência para futuros importadores.
- Não represente um componente BIM apenas como malha vazia ou etiqueta: mantenha a definição paramétrica.

### C. Vertical slice de um componente paramétrico: porta
- Use `samples/porta_parametrica.component.json` como caso real de teste.
- Na interface, adicione ferramenta simples **Porta** ou painel **Componentes** para criar uma instância a partir da definição de porta; editar o parâmetro `largura_m` por instância; recalcular grafo e atualizar uma representação geométrica **2D fiel ao parâmetro** e uma prévia isométrica simplificada (se viável nesta sprint).
- Demonstre variação de largura de `0,90 m` para `1,20 m`, incluindo resultado de cálculo coerente com o exemplo, sem recompilar.
- A edição de parâmetros deve entrar no histórico de desfazer/refazer e persistir após salvar, fechar e reabrir.
- Implementar propriedades/atributos no painel quando for possível sem desviar a sprint; não prometer renderização 3D B-Rep.

### D. Testes e documentação
- Testes para serialização round-trip, projetos legados, integridade de definições/instâncias, ciclos no grafo, divisões por zero, min/max, valores não finitos, atualização de overrides e resultado numérico após reabrir arquivo.
- Atualize `README.md` e `docs/ROADMAP.md` distinguindo **implementado**, **experimental**, **planejado** e **dependente de SDK/licença**.
- Opcional se pequeno: criar um executável de demonstração/artefato CI em Windows; não dificultar a conclusão do sprint.

## Próximas sprints — NÃO implementar como se já existissem
- Kernel CAD 3D robusto (seleção persistente de faces/arestas, booleans, B-Rep, Push/Pull e transações); comparar opções de kernel, licenças e FFI em ADR antes de integrar.
- Editor visual real de nós com sockets tipados (número, comprimento, área, booleano, vetor, geometria, lista), UI de ligações, cache incremental, geração de geometria e depuração.
- API de importação por adaptadores e relatório de fidelidade: SKP SDK oficial; IFC interoperável; RFA via APIs legítimas Revit com bridge autorizada; GSM/GDL sob mecanismos/licenças Graphisoft; Eberick inicialmente via IFC.
- Biblioteca paramétrica, catálogo, documentação executiva, quantitativos sólidos, exportação IFC, mecanismos de colaboração e subsistema estrutural independente e validado.

## Critérios de conclusão da sprint 1
- Aplicativo compila/abre em Windows 64 bits (ou registrar bloqueio técnico verificável do ambiente).
- A definição/instância de porta participa do projeto: pode ser inserida, selecionada, dimensionada dinamicamente, salva/reaberta e desfeita/refeita.
- Grafo responde à alteração `largura_m` e a geometria representada acompanha a alteração.
- Projetos anteriores continuam abrindo e testes Rust/Python passam, com saídas reais relatadas.
- Criar commits limpos com mensagens objetivas, preferencialmente em branch `feat/bim-components-sprint1`, e entregar descrição de PR com resumo, arquivos, testes, limitações.

**Comece agora pelo inventário do código, depois a compilação/testes e em seguida implemente o primeiro vertical slice completo. Não fique apenas elaborando sugestões: modifique os arquivos do repositório.**

**Regra atualizada:** não reintroduzir `Project::example`, `add_wall`, `add_slab`, `add-wall` nem a cena ilustrativa da casa piloto. O exemplo de porta é apenas teste numérico do grafo paramétrico, sem ferramenta gráfica funcional.
