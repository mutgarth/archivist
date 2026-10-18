# Validação da primeira fatia

## Verificado na implementação

- Testes de invariantes: caminhos relativos, rejeição de traversal e vetores inválidos.
- Teste de regressão: resposta de IA interrompida por limite de tokens não é aceita.
- Contrato HTTP contra PostgreSQL/pgvector real: autenticação, separação usuário/worker, importação, repetição idempotente, conflito, symlink, prévia e Range.
- Reserva concorrente: apenas uma tentativa obtém a tarefa.
- Renovação de lease, rejeição de token antigo, recuperação após expiração e limite de tentativas.
- Resultado com dimensão inválida rejeitado; conclusão válida e sua repetição aceitas; payload divergente rejeitado.
- Duas contas acessam o catálogo compartilhado; alteração do arquivo de entrada não modifica o snapshot servido.
- Busca textual e visualização funcionam com endpoint de IA indisponível; busca híbrida informa indisponibilidade.
- Navegador automatizado: login, busca híbrida, detalhe/prévia, download, logout e viewport móvel sem overflow.

`scripts/test.py` cria um banco exclusivo temporário, inicia servidor, executa `tests/api.py` e remove somente o banco criado pelo teste. Usa vetores sintéticos nas verificações de contrato; não mede qualidade do modelo.

## Inferência local

A verificação inicial com LM Studio usou imagens sintéticas de cores, Gemma 4 E4B e EmbeddingGemma de 768 dimensões. A busca recuperou a imagem azul para a consulta em português. Foi detectada descrição truncada, coberta por regressão e corrigida para rejeitar saída incompleta e permitir orçamento maior de resposta.

Isso comprova encadeamento de API/modelo/persistência/busca neste smoke test, não uma avaliação de precisão, desempenho, fotografias ou múltiplos usuários sob carga. Resultados não estabelecem requisitos mínimos de hardware.

## Executar novamente

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked
python3 scripts/test.py
```

Teste de navegador opcional com Playwright já disponível no ambiente:

```sh
ARCHIVIST_TEST_URL=http://127.0.0.1:8098 \
ARCHIVIST_TEST_TOKEN_FILE=/caminho/privado/chave \
node tests/browser.cjs
```

Exige exemplos blue.png/red.png já importados, analisados e LM Studio ativo. Chave e capturas não devem ser versionadas. O teste de browser padrão grava capturas em /tmp.

## Pendências

Corpus de fotografias autorizado e consultas PT-BR com relevância anotada; EXIF/orientação; cenários de falha de disco real; restauração de backup; carga e memória; configuração HTTPS; empacotamento do serviço; revisão de segurança mais ampla. O arquivo de entrada deve ter terminado de copiar; checagens de metadata não provam ausência de toda modificação concorrente maliciosa.

A arquitetura mantém domínio, mídia e IA em módulos próprios. Nesta primeira fatia, orquestração dos casos de uso e SQL ficam no módulo do servidor; a separação adicional será feita quando um segundo adaptador realmente a exigir.
