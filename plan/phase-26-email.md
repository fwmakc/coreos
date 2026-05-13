# Этап 26 — Email Engine

## Цель
Создать подсистему электронной почты: IMAP/SMTP клиент, синхронизация, поиск, композер. После этого этапа пользователь может отправлять и получать письма через Workspace.

## Язык и стек
- **Язык:** TypeScript
- **Runtime:** Bun
- **Ключевые зависимости:** `emailjs-imap-client` или кастомный IMAP/SMTP client, `bun:sqlite` (кэш писем, FTS5), `bun:tcp` (TLS)
- **Целевые ОС:** Windows, macOS, Linux, Android, iOS

## Зависимости
- **Этап 8** — Host Shim: Network (TCP/UDP, TLS).
- **Этап 20** — P2P Mesh (WireGuard, но email идёт через публичные серверы, не P2P).
- **Этап 25** — Messenger Engine (контакты, уведомления, UI шаблоны).

## Часть системы
**Level 1 — Бэк: Email** [См. layer-8 §6.2, layer-1 §5, layer-5 §5]

## Требования

### 26.1 Accounts
- SQLite таблица `email_accounts` (address, imap_server, smtp_server, port, encryption, credentials encrypted).
- Поддержка multiple accounts (Gmail, Outlook, Yandex, корпоративные).
- **OAuth2:** placeholder (Gmail, Outlook). Базовый IMAP/SMTP (login/password) на этом этапе.
- Credentials хранятся в Key Manager (этап 30), зашифрованные.

### 26.2 IMAP
- Подключение к серверу (STARTTLS или SSL/TLS).
- Синхронизация папок: Inbox, Sent, Drafts, Trash, Spam, Archives.
- Fetch: headers + body (lazy: body загружается по требованию).
- IDLE mode: поддержка push-уведомлений от сервера (RFC 2177), fallback на periodic poll (5 мин).
- Full-text search: FTS5 по заголовкам (From, To, Subject) и телам писем (кэшируется локально).
- Threading: группировка писем в threads (по References/In-Reply-To headers).

### 26.3 SMTP
- Отправка писем с поддержкой MIME (multipart, attachments до 25 МБ).
- Queue: если отправка не удалась — письмо ставится в очередь, повтор через 5 минут (exponential backoff до 1 часа).
- Drafts: автосохранение черновиков каждые 30 сек (CRDT-операция, синхронизируется между устройствами).
- HTML composer: WYSIWYG редактор с markdown shortcuts, inline images.

### 26.4 UI (core.email)
- Список писем: sender, subject, snippet, date, unread, thread depth.
- Чтение письма: HTML rendering через Island Mode (этап 24) с sandbox. Блокировка remote images (opt-in).
- Композер: markdown + attachments + contact autocomplete (из core.contactbook).
- Search bar: FTS5 + фильтры (дата, sender, has:attachment).

### 26.5 Notifications
- Новое письмо → системное уведомление (этап 18, Notification Manager).
- Batch notifications: если пришло 5+ писем за 5 мин — одно уведомление "5 новых писем".

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| IMAP sync | Синхронизация | Подключиться → 100 писем загружены < 30 сек |
| Send email | SMTP | Отправить письмо → в Sent, доставлено на другой аккаунт < 1 мин |
| Search FTS | Полнотекстовый | Поиск "invoice" → находит письмо < 100 мс |
| Draft autosave | Черновики | Набрать текст → через 30 сек draft сохранён |
| HTML render | Чтение | HTML письмо → отображается без remote images |
| Thread view | Цепочки | 3 письма в thread → группировка корректна |

## Интеграция с будущими этапами
- **Вход:** этап 8 (Network) — TCP/TLS.
- **Вход:** этап 25 (Messenger) — contacts, notifications.
- **Выход:** email events → этап 18 (Notification Manager).
- **Выход:** HTML body → этап 24 (Island Mode) для rendering.

## Критерии приёмки
- [ ] IMAP: синхронизация 100 писем < 30 сек.
- [ ] SMTP: отправка, получение на другом аккаунте < 1 мин.
- [ ] FTS: поиск по 1000 письмам < 100 мс.
- [ ] Draft autosave каждые 30 сек.
- [ ] HTML rendering без remote images (по умолчанию).
- [ ] Thread grouping корректна.

## Ссылки
- [layer-1-user-experience.md](../layers/layer-1-user-experience.md) — Messenger, контакты
- [layer-5-devices.md](../layers/layer-5-devices.md) — Email
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Email §6.2
