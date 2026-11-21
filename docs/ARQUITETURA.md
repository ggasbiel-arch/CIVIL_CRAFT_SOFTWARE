# Arquitetura técnica — Projeto 2

## Identidade e isolamento

O Projeto 2 é um aplicativo **independente**, desenvolvido do zero sob nova base de código. O Projeto 1 refere-se exclusivamente ao plugin para SketchUp. Requisitos, implementações, bugs e roadmaps não são compartilhados automaticamente.

## Metas de linguagem

- **Rust ~75% (meta arquitetural):** geometria, modelo de dados, comandos, interface, renderização, importação e módulos críticos.
- **Python ~25% (meta arquitetural):** automação, regras de projeto, scripts, plugins e ferramentas de processamento em lote.

Percentuais são metas, não métricas garantidas de linhas de código: dependências externas podem ser desenvolvidas em C/C++.

## Requisitos essenciais incorporados na v0.2

- Importar componentes SketchUp e permitir **Transformar em BIM** com classificação, parâmetros e propriedades.
- Importar famílias Revit, objetos Archicad e dados estruturais Eberick com conversão explícita de semântica e parâmetros recuperáveis; limites de APIs/formats reportados sem promessas irreais.
- Modelagem livre, paramétrica e procedural visual por nós no mesmo ambiente; instâncias dinâmicas por referência à definição.
- Meta de desenvolvimento 75% Rust e 25% Python.

Ver `docs/INTEROPERABILIDADE_E_PARAMETRIZACAO.md` para contratos, restrições e roadmap.

## Subsistemas planejados

1. **Core/Model:** objetos semânticos BIM (GUID, classe, propriedades, unidades, relacionamentos), estado do documento, versionamento de schema e migrações.
2. **Geometry:** sólidos exatos B-Rep, curvas, superfícies, operações booleanas, offset, extrusão, tolerâncias, topologia e tesselação. Isolar por uma interface (`GeometryKernel`) para testar integração com Open CASCADE. Não reconstruir um kernel CAD industrial como tarefa inicial.
3. **Editor:** comandos, snap, seleção, subelementos, inferências geométricas, histórico undo/redo, modo de edição direta, vistas 2D/3D.
4. **Parametric/BIM:** grafo dirigido e reativo, objetos por definição/instância, editor tipo Grasshopper/Blueprint, assistente de conversão geometria→BIM; paredes, lajes, cobertas, portas, esquadrias, escadas, fundações, propriedades, constraints, famílias, níveis e vínculos entre objetos.
5. **Documentation:** vistas associativas, plantas, cortes, elevações, folhas, cotas, símbolos editáveis e estilos de representação.
6. **Interoperability:** IFC4/IFC4.3, `.skp` via SDK oficial, `.rfa` via pipeline API Revit, `.gsm` via conversão Graphisoft quando viável, dados Eberick via IFC; DXF/DWG conforme SDK/licença; relatório de fidelidade semântica obrigatório.
7. **Structure:** geometria analítica, cargas, combinações, elementos finitos, dimensionamento, detalhamento e relatórios auditáveis, com testes contra casos de referência e supervisão de engenharia.
8. **Python automation:** interface versionada, sandbox/permissões, isolamento de processo para evitar que scripts corrompam documentos.
9. **Persistence/collaboration:** transações atômicas, journaling, backups, IDs estáveis, conflito e revisão de modelos. SQLite é candidato; `.bim.json` é somente experimental.

## Referências de bibliotecas

- GUI desktop: eframe/egui (primeiro protótipo), eventualmente docking profissional e viewport wgpu dedicado.
- Geometria exata: Open CASCADE Technology (LGPL-2.1 com exceção; revisar obrigações da versão integrada).
- IFC: especificações e MVD da buildingSMART (ISO 16739-1:2024 para IFC4.3), implementação incremental com testes reais.
- Comunicação Rust↔Python: decidir entre PyO3, IPC e bindings de C-ABI, após estabilizar as interfaces. A primeira versão usa JSON intercambiável, sem embedding.

## Decisões para revisão antes do MVP 1.0

- Windows x64 como primeiro destino; avaliar Linux/macOS depois.
- Arquivo único robusto de projeto (schema versionado, mídia e transações) substituirá JSON.
- Editor de geometria (B-Rep) e motor estrutural serão duas dependências arquiteturais independentes.
- Unidades internas SI (metros) com apresentação configurável; tolerâncias explícitas por operação.
- Nenhuma função avançada será indicada como concluída antes de testes automatizados e verificação prática.

## Propriedade intelectual

Reproduzir fluxos e capacidades gerais é diferente de reutilizar código, componentes protegidos, identidades, ativos, algoritmos não licenciados ou formatos sem autorização. Revisar termos e licenças antes de qualquer importador/exportador proprietário.
