# E2 — Plantas, Cortes, Elevações, Pranchas e PDF

## O que foi construído

1. **Núcleo geométrico de documentação** (`civil_core::documentation`): cada elemento construtivo já armazenado no arquivo BIM fornece seu contorno e arestas 3D. Paredes com segmento/espessura/altura são interpretadas como prismas retangulares; lajes poligonais como prismas extrudados em Z. Esses elementos **já existiam como dados legados**, sem comandos de criação.
2. **Plantas**: contornos 2D de paredes e lajes do pavimento selecionado, em coordenadas métricas derivadas do modelo.
3. **Cortes**: interseção de uma reta/semiplano vertical com a seção XY dos prismas, gerando retângulos no plano horizontal X / vertical Z. A posição da linha de corte e a orientação são configuráveis no Mapa do Projeto (X, Y e ângulo).
4. **Elevações**: projeção do wireframe geométrico no plano de elevação, com altura real Z; **sem remoção de arestas ocultas**, algo que requer o futuro mecanismo de visibilidade.
5. **Vistas salvas**: o vínculo `saved_view.viewpoint_id` é resolvido em tempo de visualização e publicação. Uma modificação no modelo reflete nas vistas vinculadas e no PDF exportado.
6. **Pranchas associativas**: coleção `workspace.sheets` no projeto JSON; formato inicial A3 paisagem (420 × 297 mm), numeração, nome, até quatro quadros vinculados a `saved_view_id`, pré-visualização e exportação.
7. **Publicador**: aceita publicações de folhas PDF, além de vistas JSON e tabelas CSV. Os documentos são exportados para a pasta configurada.

## Como testar na interface

- Abra o aplicativo com `cargo run --release -p civil_desktop`.
- Navegue em **Projeto**; clique em **Planta — Térreo** para exibir a representação derivada dos dados presentes no arquivo.
- Crie um **Corte** ou **Elevação**; no painel do viewpoint, ajuste X/Y e orientação em graus. A geometria é recalculada a cada visualização.
- Em **Vistas/Tabelas**, selecione um viewpoint, salve uma vista, crie uma prancha A3, selecione a vista salva e clique **Adicionar vista selecionada à prancha**.
- Use **Pré-visualizar prancha** e **Exportar prancha PDF**. A saída será criada no diretório de publicação definido no painel Publicador (padrão `publicacoes/`).
- Para publicar várias pranchas de uma vez, crie um conjunto Publicador, selecione uma prancha e adicione-a ao conjunto, então clique **Publicar conjunto**.

### Projetos sem elementos

O arquivo `samples/projeto_vazio.bim.json` não tem paredes nem lajes — continuará vazio. **Não reintroduzir a casa piloto nem as ferramentas provisórias de desenho**. A visualização de geometria funciona em projetos BIM antigos que já contenham tais entidades e em arquivos importados compatíveis com o schema v1.

## Limitações importantes

- O sistema ainda **não implementa B-Rep, geometria CSG, paredes BIM complexas, famílias paramétricas reais ou ferramentas de desenho**.
- A vista 3D pode mostrar arestas de elementos legados, mas o PDF inicial aceita apenas planta, corte, elevação e detalhe; PDF de perspectivas 3D não é suportado.
- Cortes: geração geométrica de perfil, **sem profundidade de vista, prioridades de material, hatch, representação de camadas ou substituições gráficas**.
- Elevações: arestas projetadas, **sem ocultação de arestas**.
- Os quadros na prancha usam **ajuste ao retângulo disponível** (fit-to-frame); a etiqueta de escala é metadado, **não garante escala física 1:N**. É proibido usar esta saída preliminar como desenho executivo para construção.
- PDFs têm fonte Helvetica padrão e traçados vetoriais. Ainda não há imagens, cotas, carimbo padronizado ABNT completo, escala técnica nem PDF/A.
- O tamanho A3 e os quatro quadros fixos são escolhas iniciais do MVP, não limitação permanente da arquitetura.
- Toda geometria é recalculada na abertura da vista: não há cache oculto, o que mantém a associação dos dados mas não oferece otimização para arquivos extensos.

## Critérios automatizados

- Plantas apresentam contornos derivados do modelo e filtram pelo pavimento.
- Corte muda de resultado ao deslocar o plano de corte; fora do edifício gera desenho vazio.
- Elevações projetam arestas reais.
- A prancha serializa com os vínculos preservados e rejeita referências quebradas.
- PDF inclui cabeçalho, `xref`, saída vetorial, nome/número e desenhos derivados de vistas.

## Próximas etapas

Kernel CAD de precisão; renderizador 3D com seleção de elementos reais; escala de impressão física, clipping/ocultação e estilos de desenho; cotas e anotações; editor de pranchas arrastável; layouts A0–A4; exportação IFC/DWG/DXF/PDF/A mediante validação.
