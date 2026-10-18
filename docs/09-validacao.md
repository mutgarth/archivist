# Plano de validação

> Direção de produto e plano inicial. Para o estado implementado da primeira fatia, consulte [execução](12-executar.md) e [validação](13-validacao-primeira-fatia.md).

**Plano abrangente de validação.** A cobertura já executada da primeira fatia está em [validação da implementação](13-validacao-primeira-fatia.md); os demais casos continuam pendentes.

| Área | Caso | Evidência de aprovação futura |
| --- | --- | --- |
| Domínio | Trecho inválido, caminho indevido, revisão concorrente | Rejeição previsível sem alterar estado |
| Importação | Cópia incompleta, arquivo alterado, repetição | Sem ingestão silenciosa nem duplicação |
| Filesystem | Traversal, symlink e troca concorrente | Nenhum arquivo fora da raiz acessível |
| Fila | Dois pedidos de reserva concorrentes | Uma lease ativa por tarefa |
| Retomada | Worker cai antes/depois da resposta | Recuperação idempotente e tentativas limitadas |
| Resultado atrasado | Lease expirada ou geração antiga | Não substitui geração recente |
| Correção humana | IA termina após edição | Correção preservada e vetor atualizado depois |
| Vetores | Modelo/dimensão divergente, NaN | Rejeição e índice compatível |
| Busca | Perguntas PT-BR com relevância anotada | Recall@k e latência medidos, sem meta inventada |
| Auth | Conta sem acesso, worker usando rota humana | Rejeição e ausência de vazamento |
| Arquivos | Range válido/inválido, nome malicioso | 206/416 e cabeçalhos seguros |
| Degradação | IA ou serviço externo offline | Catálogo e original continuam acessíveis |
| Recursos | Disco cheio, GPU sem memória, processo travado | Falha controlada, limpeza e erro visível |
| Backup | Restaurar banco e originais | Hashes, referências e correções conferidos |
| UI | Teclado, leitores de tela, erros e espera | Fluxo utilizável sem depender só de cor |

Após existir código: rodar `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings` e testes reais de domínio/integração. Banco de testes descartável, sem dados pessoais. Testar contratos HTTP com OpenAPI e Postman CLI quando a API existir.

Registrar versão do código/modelos, parâmetros, corpus consentido, hardware aferido e saída dos comandos. Não registrar tokens, caminhos privados ou mídia pessoal no Git. Um teste não executado deve continuar marcado como pendente.
