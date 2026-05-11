# Этап 6 — Communication Layer (Коммуникации)

> **Цель:** Работают мессенджер (P2P + внешние мосты), почта (IMAP/SMTP), звонки (VoIP), контакты, ИИ-мост. Пользователь может писать сообщения, отправлять письма, звонить.

**После этого этапа:** `@ivan` в Command Bar открывает чат, сообщения ходят между устройствами через P2P. Почта синхронизируется через IMAP. Звонки работают через WebRTC.

---

## Зависимости

- **Этап 2 (Command Bar):** Command Bar (режим мессенджера), SQLite
- **Этап 3 (Проекты):** Window Manager (окна чатов)
- **Этап 4 (Приложения):** App Registry, Permissions UI
- **Этап 5 (P2P):** P2P Mesh, CRDT (для сообщений)

---

## Компоненты

### 6.1 Contact Book (Level 1)

```typescript
// micro-kernel/src/communication/contacts.ts
export class ContactBook {
    async add(contact: ContactInput): Promise<Contact>;
    async find(query: string): Promise<Contact[]>; // по имени, нику, email, телефону
    async get(id: string): Promise<Contact | null>;
    async update(id: string, data: Partial<Contact>): Promise<Contact>;
    async delete(id: string): Promise<void>;
    async list(): Promise<Contact[]>;
    
    // Platforms
    async linkPlatform(contactId: string, platform: Platform, handle: string): Promise<void>;
    async getPlatforms(contactId: string): Promise<PlatformLink[]>;
}

interface Contact {
    id: string;
    name: string;
    nickname?: string;
    email?: string;
    phone?: string;
    avatar?: string; // URL или base64
    platforms: PlatformLink[];
    createdAt: number;
    updatedAt: number;
}

interface PlatformLink {
    platform: 'telegram' | 'whatsapp' | 'slack' | 'email' | 'core';
    handle: string; // @username, email, phone
    isPrimary: boolean;
}
```

**SQLite schema:**
```sql
CREATE TABLE contacts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    nickname TEXT,
    email TEXT,
    phone TEXT,
    avatar TEXT,
    created_at INTEGER,
    updated_at INTEGER
);

CREATE TABLE contact_platforms (
    contact_id TEXT NOT NULL,
    platform TEXT NOT NULL,
    handle TEXT NOT NULL,
    is_primary INTEGER DEFAULT 0,
    FOREIGN KEY (contact_id) REFERENCES contacts(id)
);

CREATE INDEX idx_contacts_name ON contacts(name);
CREATE INDEX idx_contacts_email ON contacts(email);
CREATE INDEX idx_contacts_phone ON contacts(phone);
```

### 6.2 Messenger (Level 1 + Level 2)

```typescript
// micro-kernel/src/communication/messenger.ts
export class Messenger {
    private chats: Map<string, Chat>;
    private p2p: P2PMesh;
    
    async sendMessage(chatId: string, content: MessageContent): Promise<Message>;
    async sendToContact(contactId: string, content: MessageContent): Promise<Message>;
    async getHistory(chatId: string, limit: number, before?: number): Promise<Message[]>;
    async markRead(chatId: string, messageId: string): Promise<void>;
    async createGroup(name: string, members: string[]): Promise<Chat>;
    
    // P2P sync
    private async onP2PMessage(peerId: PeerId, data: P2PMessageData): Promise<void> {
        const message = this.deserialize(data);
        await this.storeMessage(message);
        await this.notifyUser(message);
    }
}

interface Message {
    id: string;
    chatId: string;
    senderId: string;
    content: MessageContent;
    timestamp: number;
    editedAt?: number;
    readBy: string[]; // peer ids
}

type MessageContent = 
    | { type: 'text'; text: string }
    | { type: 'image'; fileId: string; width: number; height: number }
    | { type: 'file'; fileId: string; name: string; size: number }
    | { type: 'voice'; fileId: string; duration: number }
    | { type: 'location'; lat: number; lng: number };
```

**Шифрование:**
- End-to-end через WireGuard tunnel (уже есть)
- Дополнительно: Double Ratchet (Signal Protocol) для forward secrecy
- Ключи: X25519, derived из peer public keys

**CRDT для сообщений:**
```sql
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    chat_id TEXT NOT NULL,
    sender_id TEXT NOT NULL,
    content TEXT, -- JSON
    hlc_physical INTEGER,
    hlc_logical INTEGER,
    peer_id TEXT,
    UNIQUE(chat_id, id, hlc_physical, hlc_logical, peer_id)
);
```

### 6.3 Email Engine (Level 1)

```typescript
// micro-kernel/src/communication/email.ts
export class EmailEngine {
    private imap: IMAPClient;
    private smtp: SMTPClient;
    
    async configure(account: EmailAccount): Promise<void>;
    async syncInbox(): Promise<Email[]>;
    async sendEmail(to: string, subject: string, body: string, attachments?: Attachment[]): Promise<void>;
    async getEmail(id: string): Promise<Email>;
    async searchInbox(query: string): Promise<Email[]>;
    async markRead(id: string): Promise<void>;
    async deleteEmail(id: string): Promise<void>;
    
    // Индексация
    async indexEmails(): Promise<void> {
        const emails = await this.syncInbox();
        for (const email of emails) {
            await searchIndex.insert({
                type: 'email',
                title: email.subject,
                content: email.body,
                tags: [email.from],
            });
        }
    }
}

interface EmailAccount {
    email: string;
    password: string; // хранится в Keychain
    imapServer: string;
    imapPort: number;
    smtpServer: string;
    smtpPort: number;
    useTls: boolean;
}
```

**Протоколы:**
- IMAP4 (async-imap crate или аналог)
- SMTP (lettre crate)
- OAuth2 для Gmail / Outlook (через Auth Proxy на этапе 8)

### 6.4 VoIP (Level 0 + Level 2)

```rust
// communication/src/voip.rs
pub struct VoIPService {
    webrtc: WebRTCStack,
    audio: AudioHost,
    p2p: P2PMesh,
}

impl VoIPService {
    pub async fn call(&mut self, contact_id: &str) -> Result<Call, VoIPError> {
        let contact = self.contacts.get(contact_id)?;
        let peer_id = self.resolve_peer_id(contact)?;
        
        // Signal через P2P
        self.p2p.send_signal(peer_id, Signal::Offer { sdp: self.create_offer() }).await?;
        
        // Ожидание answer
        let answer = self.p2p.wait_for_signal(peer_id).await?;
        self.webrtc.set_remote_description(answer.sdp)?;
        
        // ICE через P2P (WireGuard уже пробил NAT)
        Ok(Call { peer_id, start_time: Instant::now() })
    }
    
    pub async fn accept_call(&mut self, peer_id: &PeerId) -> Result<Call, VoIPError> {
        // Аналогично, но с answer
    }
    
    pub async fn end_call(&mut self, call: &Call) -> Result<(), VoIPError> {
        self.webrtc.close();
        self.p2p.send_signal(call.peer_id, Signal::HangUp).await?;
        Ok(())
    }
}
```

**WebRTC stack:**
- `webrtc-rs` (Rust) или `libdatachannel` (C++ bindings)
- Audio: Opus codec через CPAL
- Video: VP8/VP9 (опционально, для видеозвонков)
- Signaling: через P2P Mesh (custom signal messages)

**Внешние звонки:**
- SIP bridge (via `rsip` crate или `pjsip`)
- GSM bridge (через провайдера SIP trunk)

### 6.5 ИИ-мост

```typescript
// micro-kernel/src/communication/ai-bridge.ts
export class AIBridge {
    async sendDocumentToContact(documentId: string, contactId: string): Promise<void> {
        // 1. Найти документ
        const doc = await searchIndex.findById(documentId);
        
        // 2. Найти контакт
        const contact = await contactBook.get(contactId);
        
        // 3. Подтверждение пользователя
        const confirmed = await permissionsUI.confirm({
            title: 'Отправить документ?',
            body: `Отправить "${doc.title}" ${contact.name}?`,
        });
        
        if (!confirmed) return;
        
        // 4. Отправка
        if (contact.hasPlatform('core')) {
            await messenger.sendToContact(contactId, { type: 'file', fileId: documentId });
        } else if (contact.hasPlatform('email')) {
            await email.sendEmail(contact.email, 'Документ', 'Во вложении', [doc]);
        } else if (contact.hasPlatform('telegram')) {
            await telegramBridge.sendDocument(contact.handle, doc);
        }
    }
    
    // "Скинь документ Ивану" → Intent API
    async handleIntent(intent: Intent): Promise<void> {
        const document = await searchIndex.findLatestDocument();
        const contact = await contactBook.findByName(intent.entities.contact);
        await this.sendDocumentToContact(document.id, contact.id);
    }
}
```

### 6.6 UI чатов

```typescript
// Рендеринг окна мессенджера
function renderChatWindow(chat: Chat, messages: Message[]): RenderCommand[] {
    const cmds: RenderCommand[] = [];
    
    // Список сообщений (scrollable)
    let y = windowHeight - inputHeight - 16;
    for (const msg of messages.slice().reverse()) {
        const isMe = msg.senderId === currentUserId;
        const bubbleColor = isMe ? '#4a9eff' : '#3a3a3a';
        const bubbleX = isMe ? windowWidth - msg.width - 16 : 16;
        
        cmds.push(drawRect(bubbleX, y - msg.height, msg.width, msg.height, bubbleColor, [12, 12, 12, 12]));
        cmds.push(drawText(bubbleX + 12, y - msg.height + 24, msg.content.text, 14));
        
        y -= msg.height + 8;
    }
    
    // Поле ввода
    cmds.push(drawRect(0, windowHeight - inputHeight, windowWidth, inputHeight, '#2a2a2a'));
    cmds.push(drawText(16, windowHeight - 20, inputText + '|', 16));
    
    return cmds;
}
```

---

## Шаги реализации

### Шаг 6.1: Contact Book (3 дня)

1. SQLite schema
2. CRUD операции
3. Platform links
4. Импорт из vCard / CSV
5. Тест: добавление, поиск, связь платформ

### Шаг 6.2: Messenger — P2P core (5 дней)

1. Message model + SQLite
2. CRDT для сообщений (LWW-Map)
3. P2P send/receive
4. Read receipts
5. Group chats (multi-party CRDT)
6. Тест: два устройства обмениваются сообщениями

### Шаг 6.3: Messenger — внешние мосты (4 дня)

1. Telegram Bot API bridge
2. WhatsApp Business API bridge (или unofficial)
3. Slack webhook bridge
4. Тест: сообщение из CORE → Telegram → ответ → CORE

### Шаг 6.4: Email Engine (4 дня)

1. IMAP client (async-imap)
2. SMTP client (lettre)
3. Email parsing (mailparse)
4. Индексация в Search Engine
5. Тест: получение 100 писем, отправка, поиск

### Шаг 6.5: VoIP (5 дней)

1. WebRTC stack (webrtc-rs)
2. Signaling через P2P Mesh
3. Audio capture/playback (CPAL + Opus)
4. Call UI (окно звонка)
5. Тест: звонок между двумя устройствами

### Шаг 6.6: ИИ-мост (2 дня)

1. Intent handler для "отправить [что] [кому]"
2. Search + Contact Book integration
3. Confirmation UI
4. Тест: "скинь документ Ивану" → находит документ → отправляет

### Шаг 6.7: UI и интеграция (4 дней)

1. Окно чата (bubble layout, scrolling)
2. Окно почты (список писем, чтение, написание)
3. Окно звонка (кнопки, таймер, mute)
4. Command Bar integration (@contact, email, +phone)
5. Тест: полный цикл отправки сообщения через Command Bar

---

## Критерии приёмки

- [ ] Contact Book: 1000 контактов, поиск < 50 мс
- [ ] Messenger (P2P): сообщение доставляется < 1 сек (локальная сеть)
- [ ] Messenger (bridge): сообщение в Telegram и ответ обратно
- [ ] Email: синхронизация 100 писем < 5 сек
- [ ] Email: отправка письма через SMTP
- [ ] VoIP: звонок между двумя устройствами, аудио < 200 мс latency
- [ ] ИИ-мост: "скинь документ Ивану" → отправка
- [ ] CRDT: сообщения не теряются при offline
- [ ] 60 FPS при прокрутке чата (100 сообщений)

---

## Placeholder'ы

| Placeholder | Замена в этапе | Примечание |
|-------------|----------------|------------|
| Только текстовые сообщения | Post-release | Изображения, файлы, голосовые |
| Нет видеозвонков | Post-release | VP8/VP9 video track |
| Email: только IMAP/SMTP | Этап 8 | OAuth2 (Gmail, Outlook) |
| VoIP: только CORE→CORE | Post-release | SIP bridge, GSM |

---

## Cross-reference

| Компонент | Слои |
|-----------|------|
| Contact Book | layer-8 §6.1, layer-1 (контакты) |
| Messenger | layer-8 §6.1, layer-1 (мессенджер) |
| Email Engine | layer-8 §6.2, layer-1 (почта) |
| VoIP | layer-8 §6.3, layer-1 (звонки) |
| ИИ-мост | layer-8 §6.4, layer-2 (Intent API) |
