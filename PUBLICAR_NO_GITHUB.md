# Como publicar o Projeto 2 no GitHub

O repositório desejado é `ggasbiel-arch/CIVIL_CRAFT_CAD_BIM`, **separado** de `CIVIL_CRAFT_WORK`.

## Opção A — GitHub CLI (automatizada)
1. Instale Git para Windows: https://git-scm.com/download/win
2. Instale GitHub CLI: https://cli.github.com/
3. No PowerShell, execute `gh auth login` e autentique sua própria conta `ggasbiel-arch` (não compartilhe token ou senha).
4. Extraia este ZIP, abra PowerShell **dentro da pasta `CIVIL_CRAFT_CAD_BIM`** e execute:
   ```powershell
   .\PUBLICAR_NO_GITHUB.ps1
   ```
O script cria o **novo** repositório **privado** na conta autenticada e publica a branch `main`. Se houver bloqueio para executar scripts, siga a opção B; não desabilite globalmente a segurança do PowerShell.

## Opção B — Criar pelo site e fazer push
1. Acesse https://github.com/new
2. Escolha o proprietário `ggasbiel-arch`, nome `CIVIL_CRAFT_CAD_BIM`, visibilidade **Private**, sem README, `.gitignore` ou licença automáticos.
3. Extraia o ZIP e abra PowerShell dentro de `CIVIL_CRAFT_CAD_BIM`.
4. Execute:
   ```powershell
   git init -b main
   git add .
   git commit -m "chore: initial independent CAD/BIM starter and Codex instructions"
   git remote add origin https://github.com/ggasbiel-arch/CIVIL_CRAFT_CAD_BIM.git
   git push -u origin main
   ```
Se o Git pedir identidade, configure localmente `git config user.name "Seu Nome"` e `git config user.email "seu-email-do-github"`, sem compartilhar senhas.

## Codex
1. Abra Codex no ChatGPT ou em https://chatgpt.com/codex, conecte a conta GitHub e selecione `ggasbiel-arch/CIVIL_CRAFT_CAD_BIM` (podem ser necessárias permissões do GitHub App para o novo repo).
2. Abra `CODEX_START_PROMPT.md` e copie **todo** o conteúdo para a primeira tarefa do Codex.
3. Peça criação de branch/PR ao invés de sobrescrever código existente sem revisão.

O primeiro sprint objetiva compilação + editor de componentes BIM paramétricos. **Não** há importadores nativos de SKP/RFA/GSM/Eberick, nem cálculos estruturais prontos neste pacote.
