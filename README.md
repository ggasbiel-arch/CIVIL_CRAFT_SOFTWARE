# CIVIL CRAFT CAD/BIM — Projeto 2

Software CAD/BIM independente, separado do Projeto 1 (plugin SketchUp).

## Estado atual — base limpa

As ferramentas provisórias de teste de **parede, janela e piso/laje** foram removidas. O starter possuía desenho de paredes por clique, prévias ilustrativas de paredes e lajes, quantitativos brutos demonstrativos e a casa piloto. Esses recursos não constituíam ferramentas BIM definitivas. Não havia uma ferramenta real de janela nessa base.

O aplicativo agora inicia com **um projeto vazio**. Mantém a interface desktop, o navegador de pavimentos, o nome do projeto e as operações de abrir/salvar; as áreas 2D e 3D estão reservadas à implementação definitiva.

Preservamos no núcleo Rust o schema experimental com tipos legados de parede/laje para **abrir arquivos antigos sem descartar seus dados**, mas sem oferecer ferramentas de criação ou renderização desses elementos. O grafo numérico paramétrico e os utilitários Python de validação também permanecem disponíveis.

**Ainda não implementado:** kernel CAD B-Rep, viewport 3D real, editor visual por nós, importadores SKP/RFA/GSM/IFC, ferramentas BIM profissionais de paredes/pisos/janelas, documentação associativa e análise estrutural validada.

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
