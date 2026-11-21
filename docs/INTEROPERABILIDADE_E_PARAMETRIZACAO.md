# Projeto 2 — Especificação v0.2: interoperabilidade, BIM, modelagem livre e programação visual

Status: **requisitos aprovados pelo usuário; arquitetura proposta; implementação parcial do esquema, do avaliador numérico e da porta nativa**.

## Objetivo funcional

Uma única definição de componente pode combinar:
- **Geometria direta:** faces/arestas/sólidos editáveis sem exigir histórico de operações para tudo.
- **Definição BIM:** classe, GUID, tipo, propriedades, materiais, unidades, quantitativos e relações construtivas.
- **Parametrização:** parâmetros de tipo/instância, restrições, fórmulas, dependências, histórico editável e regeneração.
- **Grafo visual:** nós tipados, conexões, entradas, valores, operações geométricas, condicionais e eventos controlados.
- **Origem importada:** aplicativo/versão/ID original, metadados originais, parâmetros traduzidos e relatório de perdas.

Toda importação preserva a **origem** e distingue os dados realmente interpretados de campos brutos mantidos apenas para consulta. Não declarar compatibilidade completa sem testes. Os importadores devem tratar modelos de terceiros como dados não confiáveis, limitar recursos, isolar conversores e não executar macros/scripts importados automaticamente.

## Mapa de adaptadores: escopo e limites

| Origem | Caminho de implementação previsto | Informações a preservar quando acessíveis | Limitações a documentar |
|---|---|---|---|
| SketchUp `.skp` | SDK C oficial, adapter C-ABI a Rust; leitura de grupos, definições/instâncias e dicionários | Geometria, materiais, transforms, aninhamento, atributos; campos dos componentes dinâmicos quando presentes | Fórmulas DC e Live Components podem não ser totalmente equivalentes; reconverter para grafo interno de forma explícita |
| Revit `.rfa` | Exportador/conversor externo usando Revit API onde houver Revit/licença compatível; IFC/formatos alternativos para geometria | Tipos, parâmetros, fórmulas traduzíveis, geometria, categorias, metadados, IDs | Não prometer leitura/execução nativa independente de Revit; restrições de hospedagem, fórmulas, famílias adaptativas e comportamento específicos podem perder semântica |
| Archicad `.gsm` / GDL | Ferramentas oficiais Graphisoft de conversão de libparts (XML/HSF) quando autorizadas, adaptador específico | Parâmetros, dados GDL inspecionáveis, geometria derivada, IDs e metadados | Interpretar arbitrariamente o runtime GDL seria trabalho separado; preservar scripts como referências sem executar conteúdo não confiável; observar licenças |
| Eberick | IFC e conjuntos de propriedades como primeiro caminho | Pilares, vigas, lajes, materiais, seções, quantitativos, propriedades exportadas e vínculos | Modelo de cálculo, malha FEM, armaduras, combinações, hipóteses e detalhes não são necessariamente preservados pelo IFC; arquivo proprietário nativo fora do escopo inicial |

Referências oficiais consultadas em novembro/2026:
- SketchUp SDK: https://developer.trimble.com/docs/sketchup/tools/sdk
- SketchUp Dynamic Components: https://help.sketchup.com/en/sketchup/adding-attributes
- Revit API FamilyManager: https://help.autodesk.com/cloudhelp/2026/ENU/Revit-API-MainReference/files/html/1cc4fe6c-0e9f-7439-0021-32d2e06f4c33.htm
- Graphisoft objetos GDL: https://gdl.graphisoft.com/gdl-basics/components-of-a-libpart/
- Graphisoft LP_XMLConverter: https://gdl.graphisoft.com/tips-and-tricks/how-to-use-the-lp_xmlconverter-tool/
- Eberick/IFC: https://altoqi.com.br/recursos/importacao-e-exportacao-ifc-openbim

## Contrato do objeto BIM universal

1. **ComponentDefinition** (`id`, `name`, `classification`, `parameters`, `property_sets`, `graph`, `provenance`): catálogo; pode ter múltiplas instâncias.
2. **ComponentInstance** (`id`, `definition_id`, `overrides`, `transform`): configuração e transform individual; mudança de parâmetro não altera outras instâncias.
3. **ImportProvenance**: origem, formato, versão/identificador, dados originais e avisos de conversão.
4. **ParametricGraph**: grafo dirigido acíclico; nós com IDs únicos e ligações tipadas. Avaliação determinística e cacheável, com detecção de ciclo, limites de profundidade/tamanho e valores não finitos.
5. **GeometricRepresentation** (futuro): B-Rep canônico mais malha de visualização e operações/histórico; o protótipo 0.2 ainda **não** tem esse subsistema.
6. **AnalyticalRepresentation** (futuro): propriedades analíticas e estruturais independentes da geometria e validadas com testes.

### Fluxo de 'Transformar em BIM'

1. Selecionar geometria ou componente importado.
2. Escolher categoria IFC/BIM (ex.: IfcDoor, IfcWindow, IfcBeam, IfcColumn), função, material e localização.
3. Identificar dimensões/planos de referência e eleger quais dimensões viram parâmetros.
4. Relacionar parâmetros com geometria usando restrições e/ou grafo; indicar quais operações são regeneráveis.
5. Atribuir propriedades de tipo e instância, códigos, custos opcionais e conjuntos de propriedades.
6. Simular alterações extremas, detectar conflitos e salvar a definição como componente reutilizável.
7. Exibir relatório de fidelidade: **completo**, **parcial**, **geometria + propriedades**, **não suportado**.

## Modelagem híbrida livre/paramétrica

- **Direct Edit**: arrastar vértices/faces, Push/Pull, offset, booleans, cortes e furos manuais.
- **History/Constraints**: registrar operações com IDs persistentes, vínculos topológicos, prioridades e regras de regeneração.
- **Dynamic/Visual**: editor de nós acoplado à viewport, atualizações incrementais por dependência, previews, grupos, presets, nós customizados Python e execução em Rust.
- Edições conflitantes com parâmetros devem exibir resolução de conflito. Nunca converter silenciosamente em malha morta ou eliminar parâmetros sem sinalização.
- Mais adiante: solver dimensional e geométrico, tipos de porta/família, hospedagem e interação entre componentes, verificações de unidade.

### Motor de nós, primeira prova implementada

O módulo Rust `civil_core::components` e o script Python `python/component_tools.py` demonstram **apenas um grafo numérico** com `input`, `constant`, `add`, `subtract`, `multiply`, `divide`, `min`, `max` e `clamp`. Existe detecção de ciclos, limites numéricos, parâmetros por definição e overrides por instância. A porta nativa usa a saída de largura da folha para regenerar seu contorno isométrico. O editor visual de nós e a geração de sólidos seguem planejados.

Exemplo: porta paramétrica em `samples/porta_parametrica.component.json`: largura=0,90m, altura=2,10m, folga=0,04m. O nó de subtração produz largura da folha de 0,86m; mudar largura da instância para 1,20m produz 1,16m. A definição original não é modificada.

## Responsabilidade entre linguagens

- **Rust — 75% (alvo de desenvolvimento):** modelo de dados, grafo tipado/reativo, motor CAD/B-Rep (via integração), avaliação incremental, viewport, BCF/IFC/adapters, persistência, constraints e cálculo geométrico.
- **Python — 25% (alvo de desenvolvimento):** automação de componentes e parâmetros, API de plugins, nós de script opcionais em processo isolado, ferramentas para conversão assistida, testes de integração e scripts de usuário.
- Dependências externas podem usar C/C++ ou outras linguagens; percentual é preferência de arquitetura, não contagem garantida do binário. Nunca executar Python importado de arquivos de terceiros sem permissão e isolação.

## Critérios para considerar um importador funcional

- Testes de abertura do arquivo real em versões documentadas.
- Relatório comparativo de elementos, unidades, IDs e transformações, inclusive componentes aninhados.
- Preservação dos parâmetros de tipo/instância recuperáveis, com indicação de fórmulas convertidas e não convertidas.
- Alterar um parâmetro após importação e verificar regeneração onde suportada.
- Medição de fidelidade geométrica e semântica; round-trip somente quando explicitamente implementado.
- Importação de arquivos malformados sem travar o programa.

## Política de distribuição

Compatibilidade funcional não autoriza redistribuição de bibliotecas de terceiros nem bypass de licenças. SDKs e conversores só serão integrados após avaliação dos termos aplicáveis. Marcas são citadas exclusivamente para identificar formatos de origem e fluxos compatíveis.
