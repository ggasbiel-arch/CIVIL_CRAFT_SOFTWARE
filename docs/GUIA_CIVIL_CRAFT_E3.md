# CIVIL CRAFT — Guia original de uso da Documentação Técnica E3

Este guia foi escrito para o CIVIL CRAFT e descreve somente funcionalidades implementadas ou experimentais. Não é cópia, adaptação textual ou tradução de manuais comerciais.

## 1. Preparar um projeto e uma vista

1. Abra o CIVIL CRAFT no Windows.
2. Em **Arquivo → Informações do Projeto**, preencha código, título, responsável, cliente, local, revisão e descrição. Esses dados aparecem parcialmente no carimbo da prancha.
3. Na aba **Projeto**, selecione um pavimento, corte ou elevação. Quando existirem elementos no arquivo, o programa calcula linhas da vista a partir do modelo.
4. Mova a câmera com Shift + botão do meio, arraste direito para órbita, botão do meio para pan ou roda para zoom.
5. Em **Vistas/Tabelas**, crie uma vista salva. Ela memoriza um snapshot de câmera e a referência ao ponto de vista original.

## 2. Escalas de desenho

Na aba **Vistas/Tabelas**, selecione uma vista salva e informe `1:50`, `1:100`, `1:200`, etc.

Para vistas planas, cortes e elevações, a escala do arquivo PDF será física:

- A **1:50**, um comprimento real de **5 m** deve ocupar **100 mm** no papel.
- A **1:100**, **10 m** devem ocupar **100 mm**.
- A **1:200**, **10 m** devem ocupar **50 mm**.

O código valida numeradores `1` e denominadores de `1` até `20000`. Se a geometria inteira não couber no quadro com margens de desenho, a publicação **falhará com mensagem**, em vez de reduzir automaticamente a escala ou manter a etiqueta incorreta.

Vistas 3D em perspectiva ainda usam ajuste ao quadro e a etiqueta **SEM ESCALA**. Não devem receber uma dimensão física `1:N` enganosa.

## 3. Canetas e estilos gráficos

Cada vista salva possui configurações independentes:

- **Técnico**: cor RGB da pena técnica configurada pela vista.
- **Preto e branco**: representação monocromática.
- **Conceitual**: representação azul-acinzentada para estudo.
- **Espessura da pena**: 0,05 mm a 2,0 mm.
- **Hachuras de corte**: nenhuma, diagonal 45° ou cruzada.
- **Espaçamento de hachura**: 0,3 mm a 20 mm, medido no papel.

As hachuras são traços vetoriais recortados ao retângulo que representa a seção do elemento, com espaçamento convertido corretamente segundo a escala 1:N. No momento **não** há biblioteca de hachuras por material nem prioridades de interseção; as estruturas de preenchimento são preparatórias.

## 4. Cotas automáticas

Ative **Cotas automáticas (medidas reais)** na vista salva.

Na primeira versão, são geradas cotas associativas simples a partir dos dados atuais do modelo:

- Planta: comprimento do eixo de paredes legadas e maior aresta de lajes legadas.
- Corte e elevação: altura projetada dos elementos disponíveis.
- Vistas 3D: sem cotas automáticas nesta etapa.

O valor exibido é calculado em metros. Alterar a geometria antes de publicar a prancha altera automaticamente o valor. **Não** há cadeias de cotas, seleção de testemunhos, tolerâncias, arredondamento por normas ou cotas de elementos novos do futuro kernel B-Rep. Não utilizar como documentação executiva.

## 5. Linhas ocultas

Ative **Ocultar arestas atrás dos elementos (experimental)**.

O CIVIL CRAFT passa a gerar linhas somente nos trechos amostrados que não são interceptados antes por outro sólido prismático. Essa rotina ainda tem resolução limitada, usa os perfis simplificados do arquivo BIM experimental e não é um algoritmo exato de linhas ocultas para B-Rep, famílias complexas ou superfícies curvas.

## 6. Pranchas e publicação

1. Na aba **Vistas/Tabelas**, crie a prancha A3 paisagem.
2. Selecione uma vista salva e clique em **Adicionar vista selecionada à prancha**.
3. Abra **Pré-visualizar prancha**. O aplicativo utiliza escala física, estilo e hachura da vista 2D.
4. Clique **Exportar prancha PDF** ou crie um conjunto na aba **Publicador**.
5. Confirme o diretório. O exportador gera PDF vetorial para as pranchas, JSON para vistas e CSV para tabelas.

As pranchas são associativas: alteração do modelo é refletida no próximo desenho/exportação. Atualmente não existem formatação A0–A4 completa, layout mestre, bibliotecas oficiais de carimbo, PDF/A, ocultação geométrica exata, fontes embutidas ou conformidade ABNT validada.

## 7. Ajuda no aplicativo

Abra **Ajuda → Guia de trabalho CIVIL CRAFT** para um resumo navegável dos atalhos, do mapa de projeto, das vistas, escalas, hachuras, cotas e publicador.

O mapa original de requisitos e evolução está em [Matriz funcional Archicad → CIVIL CRAFT](MATRIZ_FUNCIONAL_ARCHICAD_ORIGINAL.md).

## 8. Comandos de validação

Use Windows com Rust stable:

```powershell
cargo test --workspace
cargo run --release -p civil_desktop
python -m unittest discover -s tests -v
```

Se o projeto estiver vazio, não haverá contornos ou cotas. Isso é intencional: as antigas ferramentas demonstrativas de parede/janela/piso foram removidas e não foram restauradas.

## 9. Referências técnicas (não reproduzidas)

- [Graphisoft — opções de vista (Archicad 29)](https://help.graphisoft.com/AC/29/INT/_AC29_Help/140_UserInterfaceDialogBoxes/140_UserInterfaceDialogBoxes-68.htm)
- [Graphisoft — vistas e documentação 3D](https://support.graphisoft.com/hc/en-us/articles/52110939460881-ARCHICAD-s-3D-Views-and-3D-Documents)
- [Graphisoft — layout e publicação](https://help.graphisoft.com/AC/29/INT/_AC29_Help/070_Documentation/070_Documentation-119.htm)
