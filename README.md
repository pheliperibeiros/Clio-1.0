# Clio — Gravador de Oitivas e Audiências

[![Licença](https://img.shields.io/badge/license-EUPL--1.2-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024%20(stable)-orange.svg)](rust-toolchain.toml)
[![Alvo Principal](https://img.shields.io/badge/platform-Windows%2010%2F11%20x64-brightgreen.svg)]()

O **Clio** é um aplicativo desktop institucional desenvolvido em Rust para a gravação, preservação, integridade e documentação de oitivas e audiências no sistema de justiça brasileiro. Seu padrão institucional inicial atende às diretrizes do **GAECO** (Grupo de Atuação Especial de Combate ao Crime Organizado) do **Ministério Público do Estado do Tocantins (MPTO)**.

---

## 1. Princípios de Engenharia e Segurança Probatória

Por lidar diretamente com produção e custódia de prova em procedimentos investigatórios e processos judiciais:
* **Perda zero de prova:** Nenhuma gravação pode ser corrompida por queda inesperada de energia ou congelamento de processo.
* **Gravação Append-Only direta em disco:** O fluxo de mídia não trafega por buffers voláteis nem IndexedDB; é gravado de modo contínuo em disco com chamadas periódicas de sincronização (`fsync`).
* **Air-gapped (Sem Rede em Tempo de Execução):** O binário do Clio é completamente isolado da rede. Não possui telemetria, nem dependências de bibliotecas clientes HTTP, eliminando vetores de ataque remoto.
* **Integridade Criptográfica (SHA-256 + Cadeia de Hashes):** Cálculo incremental de hash sem sobrecarga de memória RAM, acompanhado de cadeia de hashes por blocos no diário de sessão SQLite (modo WAL, `synchronous=FULL`).

---

## 2. Estrutura do Workspace

```text
clio/
├─ Cargo.toml                  # Workspace Rust 2024, dependências centralizadas e perfis
├─ rust-toolchain.toml         # Versão e componentes da toolchain estável
├─ deny.toml                   # Auditoria rigorosa de licenças, bans e fontes
├─ assets/
│  ├─ icon.ico                 # Ícone oficial em 6 resoluções (16 a 256px)
│  ├─ header.png               # Cabeçalho institucional (MPTO / GAECO)
│  └─ templates/
│     ├─ extrato.default.j2    # Modelo padrão da Certidão Técnica de Integridade
│     └─ relatorio.default.j2  # Modelo padrão do Relatório de Oitiva
├─ crates/
│  ├─ clio-core/               # Tipos puros, validações, máquina de estados, regras de nomes
│  ├─ clio-config/             # Persistência atômica TOML, migrações v1->v2->v3, rotação de backups
│  ├─ clio-media/              # Pipeline GStreamer, 2 câmeras, 2 microfones, VU, tarja tiny-skia, wake lock
│  ├─ clio-custody/            # SHA-256 incremental, cadeia de blocos, manifesto JSON, selo local Ed25519
│  ├─ clio-store/              # Banco SQLite (WAL, synchronous=FULL), biblioteca, diário de eventos e rascunhos
│  ├─ clio-report/             # Motor minijinja sandboxed, geração de PDF, DOCX e DOTX com cabeçalho
│  └─ clio-app/                # Interface gráfica egui/eframe, orquestração e AppUserModelID
├─ xtask/                      # Tarefas de empacotamento MSI, verificação de licenças e SBOM
├─ installer/                  # Especificação WiX Toolset para geração do MSI
└─ docs/                       # Documentação técnica de spikes e modelo de ameaça
```

---

## 3. Funcionalidades Principais

1. **Dispositivos & Resiliência:**
   * Detecção dinâmica de câmeras e microfones em tempo real.
   * Suporte a até 2 câmeras (Depoente e Mesa) e 5 layouts: Lado a Lado, PiP Depoente, PiP Mesa, Câmera 1 e Câmera 2.
   * Suporte a 2 microfones com controle de ganho independente (−20 a +20 dB), medidores VU com indicação de pico e alerta de clipping.
   * Modos de áudio: *Mixagem única* ou *Faixas separadas + mixagem* (3 trilhas de áudio no contêiner para perícia).
   * Alerta caso qualquer microfone permaneça silencioso por mais de 10 segundos.

2. **Tarja Probatória Dinâmica (Burn-in):**
   * Gravada diretamente nos frames de vídeo a 1080p @ 30 fps.
   * Controle de transparência de 0% (opaco) a 100% (transparente), com contorno e sombra automáticos acima de 40% para legibilidade garantida e aviso institucional acima de 60%.
   * Posição superior ou inferior, temas de cor (Preto, Azul Institucional, Cinza Grafite) e cronômetro que desconta pausas/recessos.

3. **Cadeia de Custódia e Selo Local:**
   * Diário em SQLite criado antes do início da captura.
   * Cálculo de hash em blocos de 1 MiB sem carregar o arquivo na memória.
   * Geração de Manifesto JSON imutável selado ao término.
   * Selo local Ed25519 com chave privada mantida no cofre seguro do sistema operacional (`Credential Manager` no Windows).

4. **Modelos de Texto Editáveis (Extrato e Relatório):**
   * Edição com sintaxe amigável Jinja2 (`minijinja`) em sandbox.
   * Filtros customizados: `data`, `hora`, `data_longa`, `duracao`, `bytes`, `agrupar8`, `maiusculas`, `minusculas`.
   * Exportação fiel para **PDF**, **DOCX** (Word editável com cabeçalho em `header1.xml`) e **DOTX** (Modelo de documento Word).
   * *Nota sobre .dot:* O formato `.dot` (legado binário do Word 97-2003) não é gerado pelo Clio; usuários que demandam esse formato específico podem salvar a partir do `.dotx` gerado.

---

## 4. Como Compilar e Executar

### Pré-requisitos
* Sistema Operacional: Windows 10/11 x64 (ou Linux para desenvolvimento).
* Rust Toolchain estável (1.99+ com alvo `x86_64-pc-windows-msvc`).
* WiX Toolset v4/v5 (para geração do instalador MSI).

### Compilação de Desenvolvimento
```powershell
# Verificar formatação e lints
cargo fmt --all -- --check
cargo clippy --workspace --all-targets

# Executar todos os testes unitários
cargo test --workspace --locked

# Executar a aplicação localmente
cargo run -p clio-app
```

### Empacotamento de Release e Instalador MSI
```powershell
# Executa compilação otimizada, checagem de licenças e build do MSI
cargo run -p xtask -- package
```

---

## 5. Guia de Variáveis nos Modelos (.j2)

Os seguintes campos estão disponíveis nos modelos de Extrato e Relatório:

| Variável | Descrição / Exemplo |
| :--- | :--- |
| `orgao` | Instituição (ex.: *Ministério Público do Estado do Tocantins*) |
| `processo.numero`, `processo.tipo` | Número do procedimento e classe (*Autos*, *Inquérito*, etc.) |
| `depoente.nome`, `.qualificacao`, `.documento` | Dados da pessoa ouvida |
| `autoridade.titulo`, `autoridade.nome` | Autoridade condutora (ex.: *Promotor de Justiça Dr. ...*) |
| `responsavel_gravacao` | Nome do operador ou oficial responsável |
| `defesas` | Lista de advogados/defensores com `nome`, `tipo`, `oab`, `representa`, `presenca` |
| `participantes` | Lista com `nome`, `papel`, `documento`, `presenca` |
| `sessao.inicio`, `sessao.fim` | Datas ISO-8601 (combinar com filtros `data`, `hora`, `data_longa`) |
| `sessao.duracao_liquida` | Duração líquida descontando pausas (em segundos, usar `\| duracao`) |
| `arquivo.nome`, `arquivo.tamanho`, `arquivo.sha256` | Metadados do arquivo de vídeo gerado |
| `video.resolucao`, `video.fps`, `video.codec`, `video.container` | Parâmetros técnicos reais do pipeline |
| `audio.modo`, `audio.microfones` | Modo de áudio e lista dos microfones utilizados |
| `selo.fingerprint` | Impressão digital da chave pública do selo local Ed25519 |

---

## 6. Limitações Conhecidas e Modelo de Ameaça

Consulte o documento completo em [`docs/ameaca.md`](docs/ameaca.md) e o spike em [`docs/spike-container.md`](docs/spike-container.md).
* **Sem ICP-Brasil nativo:** O selo local Ed25519 protege contra adulteração externa do manifesto, mas não substitui a assinatura qualificada com token A3/A1 pelo promotor de justiça nos documentos gerados.
* **Sem carimbo do tempo externo:** Os carimbos de data/hora provêm do relógio local da máquina.
* **Contêiner Mestre em Matroska (`.mkv`):** Escolhido tecnicamente pela robustez absoluta a quedas de energia e capacidade append-only sem perda de dados.

---

## 7. Roteiro de Evolução Futura

1. Assinatura digital ICP-Brasil integrada diretamente no aplicativo.
2. Integração com repositório institucional e gestão eletrônica de documentos (GED).
3. Transcrição local offline de alta velocidade via modelos Whisper locais por hardware dedicado (NPU/GPU), preservando sigilo estrito das investigações.
