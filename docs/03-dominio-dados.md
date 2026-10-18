# Domínio e dados — modelo conceitual

> Direção de produto e plano inicial. Para o estado implementado da primeira fatia, consulte [execução](12-executar.md) e [validação](13-validacao-primeira-fatia.md).

Não é esquema SQL nem migração aprovada.

| Entidade | Dados e relações essenciais |
| --- | --- |
| Importação | ID, autor, lote relativo, confirmação da cópia, estado e resultados por arquivo |
| Asset | ID estável, caminho relativo, tipo detectado, tamanho, hash, versão do conteúdo, metadados técnicos |
| Trecho | Asset, início/fim em milissegundos, texto e proveniência; aplicável a áudio/vídeo |
| Artefato | Asset, tipo de prévia/derivado, caminho, hash e versão da geração |
| Análise | Asset/trecho, tipo, modelo, versão/digest, parâmetros, pipeline, entrada, geração e resultado |
| Tarefa | Operação, alvo, geração, estado, tentativas, lease, heartbeat e erro sanitizado |
| Coleção | Nome, autoria e relação muitos-para-muitos com assets |

Categorias também são múltiplas por asset. Contas e credenciais são dados de suporte à autenticação, nunca exemplos públicos.

## Invariantes

- Caminho é relativo à raiz de mídia configurada; IDs públicos não expõem caminhos físicos.
- Rejeitar caminhos absolutos, `..`, symlinks e escapes da raiz. Validar também na abertura do arquivo para reduzir troca entre validação e uso.
- Detectar formato por conteúdo/ferramenta de inspeção; extensão não é confiável.
- Confirmar término da cópia antes de registrar; verificar identidade/tamanho/hash durante ingestão e rejeitar alteração concorrente. Restringir escrita após estabilização.
- Caminho e hash ajudam a detectar repetição; hash igual não autoriza apagar ou consolidar originais automaticamente.
- Repetir uma importação com mesma chave não duplica registro; mesma chave com payload diferente gera conflito.
- Trecho tem início não negativo e fim maior que início, dentro da duração quando conhecida.
- Correções humanas ficam separadas das saídas de IA, com autor, revisão e data. Valor efetivo prioriza correção explícita.
- PATCH exige revisão esperada; correções concorrentes não se perdem silenciosamente.
- Uma análise registra a revisão do texto usado no embedding. Editar texto marca o vetor anterior como desatualizado e agenda nova geração.
- Embeddings guardam modelo, digest/versão, dimensão, normalização e revisão de origem. Comparar apenas vetores compatíveis.

## Persistência proposta

PostgreSQL com pgvector reúne catálogo, fila e vetores. Busca textual pode usar full-text do PostgreSQL com configuração para português a validar. Começar com busca vetorial exata no corpus pequeno; índice aproximado só após medir necessidade, dimensões e efeito dos filtros. Não há benchmark comparando PostgreSQL com SQLite + Qdrant.

Metadados de arquivo permanecem disponíveis quando análise falha. Artefatos podem ser reconstruídos; originais e correções humanas não. Backup deve preservar banco e originais de forma consistente, além de configuração não secreta e procedimento separado para segredos.
