# API — DRAFT, não implementada

Prefixo público `/api/v1`. JSON UTF-8. IDs opacos, datas UTC ISO 8601 e timestamps de mídia em milissegundos. Autenticação individual obrigatória; mecanismo de sessão ainda pendente. Se usar cookies, incluir proteção CSRF. TLS e autorização por recurso mesmo na rede privada.

| Método e rota | Contrato proposto |
| --- | --- |
| POST /imports | Registrar lote já copiado: batch_path relativo, copy_confirmed=true; Idempotency-Key obrigatório; 202 com import_id, job_id e Location |
| GET /imports/{id} | 200 com estado, contagens e resultados paginados por arquivo |
| GET /assets | 200 com items, next_cursor; limit padrão 25, máximo 100; filtros tipo, categoria, coleção e texto |
| GET /assets/{id} | 200 com metadados efetivos, revisão, proveniência e status de análise |
| PATCH /assets/{id} | Somente descrição, transcrição/correções e categorias humanas; revisão via If-Match; 200 e nova revisão |
| GET /assets/{id}/analyses | Histórico paginado com modelos, versões e gerações |
| GET /assets/{id}/preview | Prévia segura; 200/206 se disponível, 404 com código preview_unavailable se ausente |
| GET /assets/{id}/original | Download autenticado; 200 ou 206 para Range válido; 416 para intervalo inválido |
| POST /assets/{id}/analyses | Solicitar tipos permitidos, Idempotency-Key; 202 com job_id |
| POST /search | query, filtros, limit (1–100), cursor opcional; 200 com hits agrupados por asset e trechos |
| GET /jobs/{id} | Estado público, tentativas e erro sanitizado, sem lease/segredos |
| POST /jobs/{id}/retry | Somente falha elegível; Idempotency-Key; 202 com job_id; 409 se ativa ou concluída |

Rotas acima omitem o prefixo na tabela. `GET /health/live`: processo vivo, 200. `GET /health/ready`: banco e mídia acessíveis, 200 ou 503. IA offline aparece como dependência degradada e não bloqueia catálogo. Respostas de health não expõem configuração.

## Busca e exemplos fictícios

```json
{"query":"pessoa preparando café","filters":{"media_type":"image"},"limit":20}
```

Hit: asset_id, texto de evidência, tipo da evidência, revisão/modelo, score e trechos opcionais. Scores são ordenação, não probabilidades de acerto. Cursor opaco vinculado à consulta/filtros; mudanças no índice podem alterar resultados. Consulta limitada em tamanho, filtros enumerados e nenhuma expressão SQL recebida do cliente.

## Worker — prefixo /internal/v1

Credencial própria com escopo de worker; não reutilizar sessão de usuário. Raiz de mídia configurada localmente; resposta entrega caminho relativo autorizado.

| Método e rota | Resultado |
| --- | --- |
| POST /jobs/reserve | capabilities e worker_id; 200 com job, attempt_id, lease_token e lease_expires_at; 204 se vazia |
| POST /jobs/{id}/renew | attempt_id e lease_token; 200 com novo vencimento |
| POST /jobs/{id}/complete | attempt_id, lease_token, generation e resultado validado; 200 com result_id |
| POST /jobs/{id}/fail | attempt_id, lease_token, código e indicação de falha transitória; 200 com estado decidido pelo servidor |

Reserva tem chave de requisição para recuperar a mesma lease se a resposta se perder. Complete/fail repetidos com o mesmo payload da tentativa retornam o resultado já aceito; payload divergente ou lease expirada gera 409. Servidor decide retries, limites e geração válida. Heartbeat inicial proposto: a cada 30 s, lease de 120 s; calibrar antes de uso. Não manter transação aberta durante o trabalho.

Limitar tamanho do resultado, quantidade de trechos, dimensões e números finitos dos vetores. Artefatos não podem apontar para caminhos arbitrários do worker; transporte de arquivos derivados ao servidor é pendência contratual antes da implementação de prévias.

## Erros e concorrência

Envelope: `{"error":{"code":"invalid_path","message":"Caminho de lote inválido","request_id":"exemplo","details":[]}}`.

400: entrada inválida; 401: não autenticado; 403: sem permissão; 404: recurso ausente; 409: conflito de estado/idempotência/lease; 412: revisão divergente; 413: payload excessivo; 422: mídia não suportada; 429: limite; 503: dependência indisponível. Nunca devolver stack trace, caminho absoluto ou segredo.

PATCH rejeita campos desconhecidos e alterações de hash, caminho, modelo ou estado da tarefa. GET de arquivo revalida autorização e caminho; sanitiza Content-Disposition; usa Content-Type correto e nosniff. Range múltiplo pode ser recusado inicialmente, documentando suporte a intervalo único.

## Próximo contrato executável

Antes do backend, converter estas decisões em OpenAPI e exemplos de contrato; definir autenticação, limites de payload, transporte de artefatos e paginação exata. Exercitar com testes de integração e coleção Postman CLI. Nenhuma coleção nem teste foi executado nesta fase.
