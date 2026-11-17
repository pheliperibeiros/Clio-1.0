# Modelo de Ameaça e Limites de Garantia Probatória

**Aplicação:** Clio — Gravador de Oitivas e Audiências  
**Padrão Institucional:** GAECO / Ministério Público do Estado do Tocantins (MPTO)  

---

## 1. Premissas e Escopo

O Clio foi projetado como uma ferramenta institucional para conferir integridade técnica, padronização e rastreabilidade ao registro audiovisual de depoimentos e audiências. Por lidar com matéria probatória em processo penal e investigações de alta sensibilidade, a postura de engenharia deve ser transparente quanto ao que a tecnologia garante e o que ela **não** pode garantir.

---

## 2. O que o Clio Garante e Protege

### 2.1 Integridade Pós-Gravação (SHA-256 Incremental e Cadeia de Blocos)
* **Prevenção de Adulteração Silenciosa:** O arquivo de vídeo tem seu hash criptográfico SHA-256 gerado de forma incremental e registrado de forma imutável no Manifesto JSON e nos documentos técnicos (Extrato e Relatório). Qualquer modificação, corte, edição ou reprocessamento do arquivo após o cálculo do hash alterará o valor e tornará a violação evidente na verificação de integridade.
* **Cadeia de Hashes por Bloco:** A gravação de hashes intermediários a cada 4 MiB gravados no banco SQLite permite atestar a linha do tempo sequencial e detectar em que ponto eventual corrupção ocorreu.

### 2.2 Isolamento Total de Rede (Air-Gapped por Design)
* O binário do Clio é estritamente local. Não inclui clientes HTTP, não realiza telemetria, não se conecta a servidores de atualização e não expõe portas de rede locais. Isto anula riscos de exfiltração remota de dados ou ataques via vetor de rede no aplicativo.

### 2.3 Resiliência a Falha de Sistema (Crash Resistance)
* Diário em SQLite em modo `WAL` com `synchronous=FULL`, gravação direta em disco em fluxo append-only (`matroskamux streamable=true`) e `fsync` periódico garantem que eventos de corte de energia preservem a prova até o último segundo de áudio/vídeo gravado.

---

## 3. O que o Clio NÃO Garante (Limitações Tecnológicas Inerentes)

### 3.1 Integridade da Cadeia Física Antes do Pipeline
O Clio não pode atestar a veracidade da imagem antes que esta atinja o driver do sistema operacional:
* Se a câmera física for interceptada ou alimentada por um dispositivo de captura falso (software virtual como OBS Virtual Cam), o software não tem como discernir a origem física dos fótons.
* Por isso, a autoridade condutora e o operador continuam sendo os avaliadores presenciais da oitiva.

### 3.2 O Operador Local com Privilégios de Administrador
Se um operador mal-intencionado possuir acesso de administrador (`root` / `Administrator`) ao sistema operacional da estação:
* Ele pode substituir o executável por versão modificada;
* Ele pode alterar o relógio do sistema operacional antes da sessão iniciar;
* Ele pode acessar a chave privada do selo local gravada no cofre do sistema operacional (`Credential Manager` do Windows).
* **Mitigação Recomendada:** A estação de gravação deve possuir contas de usuário padrão para os operadores e controle de acesso estrito de TI institucional.

### 3.3 Selo Local Ed25519 vs. Assinatura Digital ICP-Brasil
* O **Selo Local Ed25519** do Clio cria uma assinatura com um par de chaves gerado na primeira execução da máquina e mantido no cofre do sistema operacional.
* **Finalidade do Selo:** Provar que o manifesto gerado naquele computador não foi editado externamente por terceiros que não possuem acesso à chave desta estação de trabalho.
* **O que ele NÃO é:** O selo local **não** substitui certificado digital qualificado no padrão da ICP-Brasil, **não** equivale à assinatura de membro do MP com token A3/nuvem e **não** possui presunção de não-repúdio conferida por autoridade certificadora credenciada. Para validade reforçada em juízo, a autoridade deve assinar digitalmente o Extrato/Relatório gerado com seu certificado ICP-Brasil.

### 3.4 Carimbo do Tempo (TSA)
* O horário registrado na tarja e nos documentos é obtido do relógio do sistema operacional no momento da captura. O Clio não consulta servidores de carimbo do tempo NTP autenticados nem carimbadoras credenciadas pela ICP-Brasil (já que a aplicação opera sem rede externa).
