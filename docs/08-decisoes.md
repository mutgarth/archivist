# Decisões e pendências

| Tema | Estado | Direção e motivo |
| --- | --- | --- |
| Produto | Aceito | Gerenciamento de mídia; edição sempre externa |
| Uso inicial | Aceito | Sistema auto-hospedado multiusuário, sem quantidade fixa de pessoas |
| Vetores | Aceito | Incluídos na primeira fatia de imagem |
| Rust modular | Aceito | Servidor/worker, domínio independente e adaptadores |
| PostgreSQL + pgvector | Proposto | Catálogo, fila e vetores em uma persistência |
| SQLite + Qdrant | Alternativa considerada | Sem comparação medida; reavaliar só com motivo concreto |
| Fila no banco | Aceito | Sem Redis/broker; leases e idempotência |
| Escrita de banco | Aceito | Exclusiva do servidor; worker por API autenticada |
| Originais estáveis | Aceito | Metadados organizam sem quebrar editores |
| NAS | Direção aceita | App portátil em container; TrueNAS como opção de implantação |
| Requisitos de infraestrutura | Pendente | Publicar após testes reproduzíveis |
| Modelos e versões | Pendente | Validar hardware suportado, português, licenças e consumo |
| Jev | Posterior | Serviço/contrato e envio externo a esclarecer |
| Backup | Necessário, pendente | Destino, frequência e restauração real |
| Autenticação | Pendente antes de implementar API | Contas individuais e worker distinto |
| Derivados do worker | Pendente antes das prévias | Definir transferência limitada e validada ao servidor |
| Licença do repositório | Pendente | Não assumir licença apenas pela visibilidade pública |

## Critério para mudar uma decisão

Registrar problema observado, alternativas, impacto em portabilidade/correções e evidência do teste. Mudança de modelo precisa de versionamento e reindexação; mudança de armazenamento precisa preservar caminhos e referências. Não usar números hipotéticos como benchmark.

## Próximas perguntas práticas

Estabelecer requisitos de hardware; validar VRAM e espaço para temporários; selecionar pequeno corpus autorizado; escolher modelo de visão e embedding; aprovar contrato de autenticação e artefatos; planejar restauração. Nenhuma dessas respostas impede a documentação inicial.
