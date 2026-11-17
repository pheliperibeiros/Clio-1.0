# Spike Técnico (Etapa 0): Comparativo de Contêineres de Mídia

**Projeto:** Clio — Gravador de Oitivas e Audiências  
**Data:** 17 de Novembro de 2026  
**Autores:** Equipe de Engenharia / GAECO-MPTO  

---

## 1. Objetivo do Spike
Avaliar e comprovar empiricamente a tolerância a falhas catastróficas (ex.: queda abrupta de energia ou finalização forçada do processo com `TerminateProcess` / `kill -9`) comparando duas abordagens de contêiner multimídia suportadas pelo GStreamer:
1. **MP4 Fragmentado** (`isofmp4mux` ou `mp4mux fragment-duration=2000`)
2. **Matroska Streamable** (`matroskamux streamable=true`)

---

## 2. Critérios de Avaliação
* **Gravação Append-Only & Flush Periódico:** capacidade de persistir blocos de áudio e vídeo continuamente em disco via `fsync` sem necessitar de reescrita retroativa do cabeçalho.
* **Sobrevivência a Queda Forçada:** integridade e reproduzibilidade do arquivo remanescente após interrupção forçada do processo sem envio de evento EOS (`End-of-Stream`).
* **Compatibilidade de Reprodução sem Reparo:** execução nativa sem transcodificação no Windows Media Player (Filmes e TV / Media Player do Windows 10/11) e no VLC Media Player.
* **Suporte a Múltiplas Faixas de Áudio:** suporte a 3 faixas simultâneas (Mix Geral, Microfone 1 - Autoridade, Microfone 2 - Depoente) com títulos e metadados independentes.
* **Complexidade de Recuperação:** necessidade de ferramentas externas ou remuxing manual para recompor a linha do tempo.

---

## 3. Metodologia de Teste e Resultados

### Cenário de Teste
* **Entrada de Vídeo:** `videotestsrc` em 1920×1080 @ 30 fps (H.264, 4000 kbps).
* **Entradas de Áudio:** Dois ramos `audiotestsrc` com deriva simulada, codificados em AAC (128 kbps).
* **Tempo de Gravação:** 60 segundos com encerramento forçado (`taskkill /F /PID <pid>`) exatamente no segundo 35.

### Matriz Comparativa

| Critério | MP4 Fragmentado (`mp4mux fragment-duration=2000`) | Matroska Streamable (`matroskamux streamable=true`) |
| :--- | :--- | :--- |
| **Comportamento em Queda** | Clusters `moof` + `mdat` são gravados em disco a cada 2 segundos. O atom `moov` não é finalizado. | Clusters EBML são gravados continuamente. O bloco de índice (`Cues`) no final fica ausente. |
| **Reprodução no VLC** | Reproduz até o segundo 34 normalmente. | Reproduz até o segundo 34 imediatamente, sem aviso ou falha. |
| **Reprodução no Windows 10/11** | **Falha ou aviso de arquivo corrompido** no Player nativo, pois o player exige parsing de cabeçalho global ou tabela de amostras completa. | **Reprodução nativa bem-sucedida** no Media Player do Windows 10/11 (com suporte nativo a MKV presente desde o Windows 10). |
| **Navegação (Seek / Scrubbing)** | Instável antes de reconstruir índice. | Reprodução sequencial fluida; seek pode ser ligeiramente mais lento pela ausência de `Cues`, mas sem travar. |
| **Múltiplas Faixas de Áudio** | Suportado, porém metadados de identificação de faixas em MP4 fragmentado têm suporte heterogêneo em players forenses. | Suporte nativo pleno e robusto a múltiplas faixas e nomes de trilha (`TrackName`). |

---

## 4. Decisão Técnica Fundamentada

**O formato adotado como Contêiner Mestre Obrigatório é o Matroska (`.mkv`) em modo `streamable=true`.**

### Justificativas:
1. **Segurança Probatória Inegociável:** O MKV foi concebido como um formato baseado em EBML (Extensible Binary Meta Language), análogo a um XML binário em streaming. Qualquer leitor padrão é capaz de ler os elementos sequencialmente até onde houver bytes íntegros, sem exigir um fechamento do arquivo.
2. **Independência de Pós-Processamento:** Se o computador desligar por corte de energia, o vídeo até o último segundo está salvo, utilizável e auditável imediatamente no Windows e no VLC.
3. **Cópia Derivada Opcional:** Caso um tribunal ou operador exija arquivo MP4 com índice otimizado para seek web, o Clio disponibiliza a funcionalidade de *Remux indexado* (cópia derivada), preservando o arquivo `.mkv` original intocado como fonte da cadeia de custódia.
