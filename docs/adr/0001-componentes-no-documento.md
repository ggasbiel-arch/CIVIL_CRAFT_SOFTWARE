# ADR 0001 — Definições e instâncias no documento BIM

## Decisão

O `Project` armazena definições por ID em `component_definitions` e instâncias vinculadas a um pavimento em `component_instances`. Os campos são opcionais no JSON de versão 1, o que permite abrir os modelos anteriores. Uma cópia do documento contém tudo que a porta precisa para voltar a ser editável após a reabertura. O salvamento grava um arquivo temporário no diretório de destino e só então substitui o arquivo final.

A definição inicial vem do exemplo versionado. Instâncias da mesma definição têm transformações e valores próprios. Ao confirmar a largura, o núcleo valida o novo valor, avalia o grafo e gera a geometria antes de aceitar a alteração. A planta projeta os extremos do vão nominal e a isometria projeta o contorno da folha. A posição e a cota de pavimento entram na transformação da geometria.

O histórico do editor guarda instantâneos do documento. Uma inserção ou uma aplicação válida de largura gera uma entrada. Valores rejeitados e reaplicações do valor atual mantêm o histórico como estava. O arquivo salva o estado atual.

A CLI Python preserva o catálogo e as instâncias quando adiciona paredes. Ela verifica IDs e referências. A avaliação do grafo e a validação geométrica do documento ficam no núcleo Rust.

## Verificação

Os testes Rust exercitam o compartilhamento da definição, a regeneração por instância, os limites da largura, referências inválidas, arquivos anteriores e o ciclo de salvar, reabrir, desfazer e refazer do editor. O roteiro de verificação pela janela está no README.

O próximo passo da porta arquitetônica é vinculá-la à parede e calcular o recorte. Os quantitativos de paredes seguem brutos.
