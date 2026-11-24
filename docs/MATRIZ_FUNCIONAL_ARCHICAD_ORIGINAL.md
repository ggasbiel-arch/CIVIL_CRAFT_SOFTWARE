# CIVIL CRAFT — Matriz original de capacidades CAD/BIM (referência funcional Archicad)

> **Aviso legal e técnico:** este documento é uma especificação **original** do CIVIL CRAFT. Não é uma cópia, transcrição, tradução integral, espelho, nem redistribuição do manual, dos recursos visuais, do conteúdo ou do código da Graphisoft. A implementação deve ser independente, respeitando direitos de propriedade intelectual, contratos de licença, marcas e formatos nativos.

## Objetivo

Estudar o funcionamento público de ferramentas CAD/BIM consolidadas para definir **comportamentos equivalentes** em Rust/Python e uma interface própria. O projeto continua independente de plugins SketchUp (Projeto 1).

As referências são documentações públicas: 
- [Archicad 29 — opções de vista, sobreposições e dimensionamento](https://help.graphisoft.com/AC/29/INT/_AC29_Help/140_UserInterfaceDialogBoxes/140_UserInterfaceDialogBoxes-68.htm)
- [Archicad 29 — configurações do layout](https://help.graphisoft.com/AC/29/INT/_AC29_Help/140_UserInterfaceDialogBoxes/140_UserInterfaceDialogBoxes-71.htm)
- [Archicad 29 — publicação](https://help.graphisoft.com/AC/29/INT/_AC29_Help/070_Documentation/070_Documentation-119.htm)
- [Graphisoft — conceitos do navegador](https://support.graphisoft.com/hc/en-us/articles/52110708938897-Archicad-Basic-Concepts-The-Navigator)
- [Archicad 29 — combinação de opções rápidas](https://help.graphisoft.com/AC/29/INT/_AC29_Help/020_Configuration/020_Configuration-20.htm)

*Os links são referências a consultar, não conteúdos a copiar.*

## Status

- **Base**: existente no projeto, parcialmente funcional, sujeito a teste.
- **E3**: implementação inicial nesta branch, com limitações descritas.
- **Pendente**: ainda não implementado; necessário para equivalência funcional.
- **Futuro**: exige outras camadas anteriores e validação adicional.

## 1. Projeto e dados centrais

| Capacidade original CIVIL CRAFT | Status | Aceitação mínima |
|---|---|---|
| Arquivo nativo e migração de versões | Base | Abrir projetos antigos sem perda de dados; migrar explicitamente |
| Identificação, cliente, responsável, endereço e revisões | Base | Salvar/abrir metadados e preencher carimbo |
| Pavimentos e cotas de referência | Base | Editar, reordenar, excluir com integridade referencial |
| Sistema de coordenadas global/local | Pendente | Origem e UCS salvos por vista/projeto |
| Unidades e preferências de dimensão | Pendente | Converter/formatar sem alterar coordenadas métricas |
| Ambientes de trabalho e perfis | Pendente | Persistir atalhos, menus, painéis e perfis |
| Favoritos e presets independentes | Pendente | Biblioteca global + por projeto com IDs versionados |
| Camadas, combinações e bloqueios | Pendente | Visibilidade por vista, sem perder entidades |
| Fases de obra, reformas e opções de projeto | Futuro | Alternâncias preservam modelo principal |
| Referências externas e módulos associados | Futuro | Recarregar e comparar alterações sem corromper referências |
| Auditoria e recuperação automática | Futuro | Journal de operações, checkpoints e rollback |

## 2. Editor, seleção e geometria livre

| Capacidade | Status | Aceitação mínima |
|---|---|---|
| Viewport 2D/3D e múltiplas janelas | Base parcial | Cada viewport possui câmera/corte próprio |
| Ortogonal, perspectiva, dois pontos | Base | Testar paralelismo e matrizes de visão |
| Orbit/Pan/Zoom e enquadramento | Base | Movimento incremental; gravar câmera |
| Seleção de janela e cruzamento por direção | Base parcial | Inclusão completa e por interseção de entidades |
| Seleção por filtros, atributos e ID | Pendente | Consulta determinística sobre conjunto de entidades |
| Snap em arestas, pontos, planos e eixos | Pendente | Precisão dimensional e prioridade documentada |
| Guias/grades dinâmicas e inferência geométrica | Pendente | Alinhamentos reproduzíveis e testáveis |
| Edição direta de faces/vértices/arestas | Pendente | Operações sem destruição do sólido |
| Push/Pull, Offset, Follow Me/Extrusão | Pendente | Histórico editável e atributos preservados |
| Booleanas com B-Rep robusto | Pendente | Volume, faces e IDs topológicos estáveis |
| Transformações, arrays e espelhamento | Pendente | Cópias vinculadas e independentes |
| Restrição geométrica e solução paramétrica | Pendente | Detecção de ciclos/conflitos e rollback |
| Eixos arbitrários de trabalho e planos de corte | Pendente | Manipuladores precisos com preview real |

## 3. Elementos BIM de arquitetura

| Capacidade | Status | Aceitação mínima |
|---|---|---|
| Paredes simples/compostas/perfis complexos | Pendente | Camadas e quantitativos corretos |
| Junções L/T/X por prioridade de materiais | Pendente | Sem interpenetração e sem perder grupos |
| Portas/janelas hospedadas | Pendente | Vãos seguem posição e mudança de parâmetros |
| Lajes e coberturas com contorno associativo | Pendente | Atualização após edição de contorno/espessura |
| Escadas e guarda-corpos paramétricos | Pendente | Regras geométricas e parâmetros de segurança |
| Fachadas-cortina, montantes e painéis | Pendente | Controle por grade e regras de substituição |
| Pilares, vigas e malhas estruturais | Pendente | Perfis e materiais com continuidade BIM |
| Zones/Espaços e calculadora de áreas | Pendente | Cálculo poligonal auditável e relações com paredes |
| Objetos BIM de mobiliário e componentes genéricos | Pendente | Parâmetros de tipo/instância e classificação |
| Terrenos, malhas e topografia | Pendente | Coordenadas, curvas de nível e volumes de corte/aterro |
| Aberturas e operações de sólidos | Pendente | Dependências hospedadas e interseções |
| Elementos MEP: tubulações, eletrocalhas e conexões | Futuro | Conexões e rota com regras de compatibilidade |

## 4. Família, script e automação visual

| Capacidade | Status | Aceitação mínima |
|---|---|---|
| Definições e instâncias com metadados | Base | Uma definição → várias instâncias independentes |
| Grafo de parâmetros numéricos | Base | Avaliar nós sem ciclos ou efeitos colaterais |
| Editor de famílias com restrições | Pendente | Edição visual de formas e parâmetros |
| Visual scripting estilo nós | Pendente | Grafo tipado, execução incremental e cache |
| Tipos de parâmetros, unidades e expressões | Pendente | Verificação dimensional e mensagens úteis |
| Nós geométricos (criar face, extrudir, perfurar) | Pendente | Retorno de IDs estáveis e testes de regressão |
| Suporte Python restrito/sandbox | Futuro | Permissões, quotas, logs e isolamento |
| Bibliotecas compartilhadas e versionamento | Futuro | Sincronização, migração e conflitos previsíveis |
| Importação de objetos SKP para classificação BIM | Futuro | Associar atributos e converter sem perder origem |
| Adaptação de famílias RFA/GDL via APIs legais | Futuro | Relatório por parâmetro e aviso de conversões |
| Compatibilidade IFC e elementos Eberick | Futuro | Psets/classificações com auditoria |

## 5. Documentação e apresentação

| Capacidade | Status | Aceitação mínima |
|---|---|---|
| Mapa de Projeto, Vistas e Publicador | Base | Persistência e navegação por IDs |
| Plantas de pavimento por geometria | Base parcial | Contorno correto por nível em modelo legado |
| Cortes e elevações associados | Base parcial | Recomputar quando geometria muda |
| Perspectivas 3D salvas e wireframe em prancha | Base parcial | Captura com parâmetros da câmera |
| Escala real 1:N no PDF 2D | E3 | 1 m a 1:100 mede 10 mm no papel |
| Preferências por vista: espessuras e aparência | E3 | Cada vista persiste estilo sem afetar outras |
| Hachura 2D de superfícies cortadas | E3 | Segmentos vetoriais com espaçamento em mm no papel |
| Cotas automáticas derivadas | E3 | Medidas respondem a mudanças geométricas |
| Ocultação de arestas em 3D/elevações prismáticas | E3 | Filtrar linhas obstruídas; admitir amostragem |
| Cotas associativas profissionais com eixos e cadeias | Pendente | Pontos de referência persistentes e precisão correta |
| Níveis, chamadas, marcas de corte e elevação | Pendente | Representação e referências bidirecionais |
| Etiquetas, textos, símbolos, legendas e autotextos | Pendente | Conteúdo vinculado a entidades/propriedades |
| Canetas e padrões de preenchimento customizáveis | Pendente | Mapas por material/classificação/vista |
| Tabelas de quantitativos e índices de desenhos | Base parcial | Atualização automática e formatos auditáveis |
| Pranchas A0–A4 / tamanhos livres | Pendente | Dimensões e margens persistentes |
| Editar posicionamento/recorte/rotação de desenho | Pendente | Clipping e snapping precisos |
| Layouts mestres e carimbos parametrizados | Pendente | Reuso, alterações propagadas |
| Conjuntos publicadores com PDFs agrupados | Pendente | Ordenação, nomeação, revisão, exportação em lote |
| PDF/A, PDF vetorial auditável e fontes embutidas | Pendente | Validação com renderizador e PDF/A checker |
| Exportação DXF/DWG com tradução de estilos | Futuro | Contratos/licenças e testes de interoperabilidade |
| Regras ABNT aplicáveis e perfis de documentação | Futuro | Verificação por norma e data da edição; nunca afirmar conformidade sem laudo |

## 6. Informação, colaboração e integração

| Capacidade | Status | Aceitação mínima |
|---|---|---|
| Propriedades BIM por instância/material | Base parcial | Mapear IDs, local e dimensões |
| Quantitativos de áreas/comprimentos/volumes | Pendente | Comparar contra casos de referência |
| Classificações e conjuntos de propriedades IFC | Pendente | Trocas com modelos de terceiros preservam GUIDs |
| Mapeamento IFC import/export completo | Futuro | buildingSMART IFC4/IFC4x3, round-trip auditável |
| Detecção de interferências e issues | Futuro | Relatório de colisões com elementos e vistas |
| BCF, revisão e coordenação BIM | Futuro | Identificadores e histórico de comentários |
| Trabalho multiusuário e reservas | Futuro | Controle de versão e conflitos transacionais |
| Arquitetura estrutural / análise com validação | Futuro | Cálculos testados segundo normas atualizadas |
| Sincronização com revisão de Revit/ArchiCAD/Eberick | Futuro | Compatibilidade declarada por formato/versão |

## Regras permanentes para o Codex

1. Nunca copiar manual, textos, ícones, biblioteca, arquivos nativos privados ou código-fonte proprietário. Pode consultar documentação pública para entender requisitos e escrever implementações próprias.
2. Antes de começar uma capacidade, criar contrato Rust, persistência, testes de unidade, pelo menos um teste end-to-end e documentação original em português.
3. Documentação associativa deve partir do mesmo modelo. Nada de desenhar paredes de teste no aplicativo ou de simular ferramenta com botões inertes.
4. Sempre distinguir ferramenta funcional, protótipo e backlog.
5. Para importação, manter proveniência e converter apenas dados legal/tecnicamente acessíveis; apresentar diagnóstico de perdas.
6. Manter Rust como motor principal e Python como camada de automação, com meta aproximada 75%/25%.
7. Não declarar conformidade legal ou norma técnica sem teste e documentação de evidência.
8. Todo incremento entra via PR com CI Rust/Windows e testes Python. Nunca mesclar código que não compila.
9. Quando uma capacidade depende do kernel CAD, implementar apenas infraestrutura e marcar dependência, sem alegar entrega total.

## Prioridade de execução

1. **E3**: fechar escalas físicas, estilos de pena, hachuras, cotas derivadas e remoção experimental de arestas ocultas.
2. **E4**: introduzir motor geométrico preciso, IDs de topologia estáveis e renderizador 3D.
3. **E5**: ferramentas BIM definitivas e edição direta associativa.
4. **E6**: layouts mestres, escalas técnicas, cotas completas e publicador profissional.
5. **E7**: importadores e exportadores interoperáveis, mais fluxo de QA.
6. **E8+**: automação visual, computação estrutural e colaboração.
