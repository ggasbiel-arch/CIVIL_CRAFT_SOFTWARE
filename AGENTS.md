# AGENTS.md — CIVIL CRAFT CAD/BIM | PROJETO 2

## Identidade e isolamento
Este repositório contém **somente o Projeto 2**: software independente de CAD/BIM para Windows x64. É diferente do **Projeto 1**, uma extensão para SketchUp. Não importar código, decisões ou dependências do Projeto 1 automaticamente. Não modificar `ggasbiel-arch/CIVIL_CRAFT_WORK`.

## Objetivo de produto
Unificar, em arquitetura original, os seguintes paradigmas:
- modelagem livre (faces, arestas, sólidos, push/pull, edição direta);
- BIM semântico/paramétrico (definições, instâncias, tipos, classificação, propriedades, unidades, restrições, relações, quantitativos);
- programação visual procedural por nós (inspirada funcionalmente em Grasshopper/Blueprint);
- interoperabilidade com componentes SketchUp SKP, famílias Revit RFA, objetos Archicad GSM/GDL e modelos estruturais Eberick/IFC, respeitando as capacidades de SDKs e licenças;
- documentação, tabelas e no futuro análise estrutural especializada e validada.

**Não** copiar integralmente manuais, guias, textos, código, marcas, interfaces idênticas, assets ou algoritmos proprietários de softwares de terceiros. Criar guias originais baseados em funcionalidade pública, respeitando direitos autorais. Importar dados autorizados, não burlar criptografia, licenças ou formatos protegidos.

## Stack e divisão de responsabilidades
- **Rust — aproximadamente 75% do desenvolvimento de lógica e código-fonte de produto**: domínio geométrico, sistema BIM, parâmetros, avaliação de grafos, persistência, importadores, UI desktop, operações pesadas e testes.
- **Python — aproximadamente 25% como meta, não quota artificial**: automações, interface de scripting, plugins, conversões auxiliares, batch jobs, testes e API de extensibilidade; núcleo Rust permanece fonte da verdade.
- Windows 64 bits, Rust stable (edition 2024). Manter `Cargo.toml` workspace com `civil_core` e `civil_desktop`. Preservar o protótipo e a UI atual em `eframe/egui` durante a primeira etapa. Mudanças de framework exigem ADR documentado.
- Evitar dependências pagas sem aprovação. Integrar kernel CAD 3D robusto apenas em fase posterior, mediante avaliação documentada (B-Rep, tolerâncias, licenças, interoperabilidade, segurança e performance).

## Estado real do código
`crates/civil_core` preserva paredes e lajes como tipos legados para compatibilidade de arquivos. `crates/civil_core/src/components.rs` contém definições/instâncias e um grafo **numérico**, sem geração de sólidos. `crates/civil_desktop` contém câmeras, gestão de vistas, pranchas e pré-visualização documental, sem ferramentas temporárias de criação arquitetônica. `crates/civil_core/src/documentation.rs` deriva geometrias prismáticas legadas, e `technical.rs` adiciona escalas/estilos/hachuras/cotas e ocultação aproximada. `python/` tem utilitários. Ainda não existem importadores reais, editor gráfico de nós, kernel B-Rep, exportação IFC nem cálculo estrutural validado.

## Processo obrigatório de implementação
1. Ler `README.md`, `docs/ARQUITETURA.md`, `docs/ROADMAP.md`, `docs/INTEROPERABILIDADE_E_PARAMETRIZACAO.md` e este arquivo.
2. Inspecionar os módulos existentes antes de criar novos. Corrigir defeitos de compilação antes de prometer novas capacidades.
3. Trabalhar em passos pequenos, verticalmente completos, com testes e alterações mínimas. Cada commit/PR deve ter critérios de aceitação claros.
4. Preferir modelos tipados, esquema versionado, UUID/IDs estáveis, serialização confiável, transações/undo/redo, erros explícitos e migrações compatíveis.
5. Separar `Definition` (parâmetros globais/tipo), `Instance` (transformação, overrides), `Geometry`, `Properties`, `Provenance` e `Document`. Não colar geometria importada permanentemente em representação que impeça reatribuir metadados BIM.
6. Grafo paramétrico: avaliar determinística e incrementalmente, detecção de ciclos, validação de unidades e tipos, validação de min/max, impedimento de NaN/Infinity, limite de nós/profundidade, erros inteligíveis, invalidação de cache e atualização dos dependentes. Não executar código arbitrário de origem externa.
7. Toda edição livre que possa quebrar parâmetros precisa preservar informação e apresentar opção de conversão, reparo ou desconexão explícita. Nunca descartar metadados silenciosamente.
8. Importadores terão interface extensível com relatório de fidelidade (geometria, materiais, parâmetros, fórmulas, unidades, origem, IDs, avisos). Preservar atributos originais quando permitido e possível; não prometer importação 100% fiel de formatos nativos. `RFA` e `GSM/GDL` dependem de APIs/ambientes/licenças específicos, `Eberick` prioriza IFC até confirmação de outro acesso legítimo.
9. Nenhum resultado de análise estrutural deve ser apresentado como seguro ou normativamente conforme sem motor de cálculo especializado, testes de referência, auditoria e verificação humana.
10. Documentar decisões de arquitetura com ADRs (`docs/adr/`); não reescrever toda a base sem necessidade.

## Verificação obrigatória
Executar, quando disponível:
```
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
python -m unittest discover -s tests -v
python python/civil_tools.py validate samples/projeto_vazio.bim.json
```
Se faltar ambiente/toolchain, declarar precisamente o que não pôde executar; não fingir validação. Não eliminar testes para mascarar falhas.

## Entregas por interação Codex
No fim, fornecer: arquivos modificados, resumo do que funciona, testes realmente executados e resultados, limitações reais, próximos passos. Evitar longos planos sem implementar código. Responder em português do Brasil.

## Limpeza das ferramentas de teste — novembro/2026

Não restaurar o desenho temporário de paredes, a visualização simplificada de paredes e pisos/lajes, os controles de espessura/altura da demonstração, o comando `add-wall` ou a casa piloto. Não havia ferramenta real de janela nesta base. Manter os dados legados somente para leitura de projetos antigos. Ferramentas BIM definitivas exigem geometria real, testes, persistência e edição associativa.

## Documentação E3 e propriedade intelectual

A referência funcional deve ser feita por [MATRIZ_FUNCIONAL_ARCHICAD_ORIGINAL.md](docs/MATRIZ_FUNCIONAL_ARCHICAD_ORIGINAL.md). O [GUIA_CIVIL_CRAFT_E3.md](docs/GUIA_CIVIL_CRAFT_E3.md) é texto original. Não copiar o manual do Archicad nem outros documentos protegidos, integralmente ou em grandes trechos. Implementar funcionalidades equivalentes com desenhos, algoritmos, interfaces e textos próprios.

Nunca afirmar que o PDF E3 possui escala física quando estiver em perspectiva 3D (etiqueta SEM ESCALA), ou que a ocultação por amostragem é completa. Se a vista 2D ultrapassar seu quadro à escala 1:N, interromper exportação em vez de recalcular escala silenciosamente.
