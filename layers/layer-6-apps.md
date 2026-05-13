# Layer 6 — Apps | Модель приложений

Как приложения работают в Workspace. 5 уровней интеграции — от нуля усилий до полного натива. Разработчик выбирает сам.

---

## Принцип

Workspace не заставляет разработчика переписывать приложение. Любой веб-сайт уже работает (уровень 1). Разработчик решает сам — насколько глубоко интегрироваться. Каждый уровень добавляет возможности, но требует больше усилий.

```
Уровень 1        Уровень 2        Уровень 3        Уровень 4        Уровень 5
"Как есть"       "Манифест"       "Бэк на месте"   "@core/*"        "Полный натив"

  0 усилий        5 минут          1 день           1 неделя         1 месяц

  Веб-сайт ──────→ Приложение ────→ Приложение ────→ Приложение ────→ Приложение
  в окне            с иконкой        с локальным      без бэка         WebGPU
                                     бэком                             натив
```

---

## Уровень 1: «Как есть» — ноль усилий

Любой сайт открывается как окно в Island Mode. Уже работает, разработчик ничего не делает.

**Как:**
- Пользователь набирает URL в Command Bar → `youtube.com` → открывается как окно в проекте
- Island Mode (Chromium sandbox) рендерит сайт
- Сайт стучится на свой сервер как обычно
- Пользователь может закрепить сайт как иконку на главном экране

**Что видит пользователь:**
- Окно с сайтом, как вкладка браузера, но без браузера
- Адресная строка есть (в рамках Island Mode)
- Сайт работает как есть — авторизация, API, всё через интернет

**Ограничения:**
- Нет доступа к файлам, контактам, уведомлениям Workspace
- Нет интеграции с проектами, тегами, поиском
- Закрыл окно — состояние не сохраняется (зависит от самого сайта)

**Разработчик:** ничего не делает. Его сайт уже работает.

---

## Уровень 2: «Манифест» — 5 минут

Разработчик добавляет один файл `core.json` в корень сайта. Workspace воспринимает сайт как приложение.

### Манифест

```json
{
  "name": "Todoist",
  "short_name": "Todoist",
  "description": "Task manager",
  "icon": "/icon-192.png",
  "url": "https://todoist.com",
  "display": "standalone",
  "permissions": ["notifications"]
}
```

### Что меняется

| Параметр | Уровень 1 (без манифеста) | Уровень 2 (с манифестом) |
|----------|--------------------------|-------------------------|
| Адресная строка | Видна | Скрыта — standalone-окно |
| Иконка в каталоге | Нет | Да, с именем и иконкой |
| Уведомления | Только внутри окна | Workspace push-уведомления |
| Запуск | Набрать URL | Набрать имя «Todoist» |
| Внешний вид | Как вкладка браузера | Как нативное приложение |

**Permissions** — что приложение запрашивает у пользователя:

| Permission | Что даёт | Спрашивает пользователя? |
|-----------|---------|------------------------|
| `notifications` | Workspace push-уведомления | Да, при первом запуске |
| `fullscreen` | Полноэкранный режим | Нет |

По умолчанию — никаких разрешений. Приложение — обычный сайт в standalone-окне.

**Разработчик:** добавил `core.json` в корень сайта. Всё. Аналог `manifest.json` для PWA.

---

## Уровень 3: «Бэк на месте» — 1 день

Разработчик хочет, чтобы приложение работало **оффлайн**. Бундлит свой бэкенд вместе с приложением. Фронт стучится на `localhost:PORT` — но это бэкенд, запущенный Workspace на устройстве пользователя.

### Манифест

```json
{
  "name": "Todo App",
  "short_name": "Todo",
  "description": "Local-first todo manager",
  "icon": "/icon-192.png",
  "frontend": "dist/index.html",
  "backend": "server.js",
  "port": 8321,
  "network_whitelist": ["api.todoapp.com"],
  "permissions": ["notifications"]
}
```

### Что происходит при запуске

```
Пользователь открывает "Todo App"
  |
  v
Workspace App Runtime:
  1. Создаёт V8 Isolate (Bun) для backend
  2. Запускает server.js → поднимается на localhost:8321
  3. Создаёт Island Mode (Chromium) для frontend
  4. Открывает dist/index.html
  5. Фронт ходит на http://localhost:8321/api/... как обычно
  |
  v
Результат:
  +-- Фронт (React/Vue/что угодно) → Island Mode
  +-- Бэк (Express/Fastify/Hono) → V8 Isolate на Bun
  +-- БД → app-scoped SQLite (предоставляет Workspace)
  +-- Файловое хранилище → app-scoped VFS (предоставляет Workspace)
```

### App-scoped SQLite

Бэкенд приложения получает **свою** SQLite базу, изолированную от других приложений и от данных Workspace:

```
~/.core/apps/
├── com.todoapp/
│   ├── frontend/     ← dist/index.html и статики
│   ├── backend/      ← server.js
│   ├── data.db       ← app-scoped SQLite
│   └── core.json     ← манифест
├── com.habittracker/
│   ├── ...
│   └── data.db
```

Бэкенд работает с SQLite как обычно (Bun встроенная поддержка):

```typescript
import { Database } from 'bun:sqlite'
const db = new Database('data.db')  // ← app-scoped, автоматически
db.query('CREATE TABLE IF NOT EXISTS todos (id TEXT, title TEXT, done INTEGER)')
```

### Что доступно бэкенду (sandboxed)

| Есть | Нет |
|------|-----|
| SQLite (app-scoped) | Host OS filesystem |
| Сеть (HTTP/HTTPS) — только домены из whitelist в `core.json` | Данные других приложений |
| Bun API (path, crypto, util) | Данные Workspace (контакты, проекты) |
| Environment variables | System processes (child_process) |
| Таймеры, Scheduled tasks | Произвольные порты (только выделенный) |

### ⚠️ Безопасность уровня 3

Уровень 3 — отдельная модель безопасности с повышенным риском по сравнению с уровнями 4–5:

- **Сеть не свободна.** Доступ только к доменам из `network_whitelist` в манифесте. Не указан — доступ запрещён. Бэк логирует все исходящие запросы в аудит.
- **Firewall:** только frontend Island этого приложения может обращаться к `localhost:PORT`. Другие приложения (уровень 1–2) не могут стучаться на чужой локальный порт — блокируется на уровне ОС (iptables / WFP / pf).
- **OAuth внешних сервисов** проходит через Бэк (Auth Proxy), а не через приложение. Уровень 3 имеет собственный сетевой стек и не может использовать системный Auth Proxy напрямую.
- **Network audit:** все HTTP-запросы логируются в аудит (категория «apps»). Owner видит, куда приложение стучится.

> Подробно: защита от эксфильтрации, whitelist, firewall, network audit — в [Layer Security, §10](layer-7-security.md).

### Порт

Workspace выделяет один порт для бэкенда приложения (указан в манифесте). Бэкенд слушает на `localhost:PORT`. Фронт ходит туда. Внешние устройства **не могут** подключиться к этому порту — только локально.

### Что меняет разработчик

1. PostgreSQL → SQLite (встроенная в Bun)
2. Добавить `core.json` с `backend` и `port`
3. Убрать удалённый сервер из конфига фронтенда (заменить на `localhost:PORT`)

Код фронтенда **не меняется**. Код бэкенда **минимально** (только БД).

---

## Уровень 4: «@core/*» — 1 неделя

Workspace стала популярной. Разработчик решает убрать бэкенд и использовать Workspace API напрямую.

### Что доступно

```typescript
// Данные
import { readFile, writeFile, readdir } from 'fs'      // перехвачено → Back API
import { listNotes, createNote, updateNote } from '@core/notes'
import { addTag, removeTag, getTags } from '@core/tags'
import { search } from '@core/search'

// Контакты и коммуникации
import { findContact, listContacts } from '@core/contacts'
import { sendMessage } from '@core/messenger'

// Проекты и структура
import { getProject, listProjects } from '@core/projects'
import { createTask, listTasks } from '@core/tasks'

// Уведомления
import { notify } from '@core/notifications'

// App-scoped хранилище
import { db } from '@core/db'                           // app-scoped SQLite

// Интеграция с системой
import { registerIntent } from '@core/intents'
```

### @core/db — app-scoped SQLite

Для кастомных структур данных, которые не вписываются в встроенные сущности (заметки, файлы, теги):

```typescript
import { db } from '@core/db'

// Создание таблиц — только внутри app scope
db.run('CREATE TABLE IF NOT EXISTS habits (id TEXT, name TEXT, streak INTEGER)')

// CRUD
const habits = db.query('SELECT * FROM habits WHERE streak > 0').all()
db.run('INSERT INTO habits (id, name, streak) VALUES (?, ?, ?)', [id, 'Спорт', 5])

// Транзакции
db.transaction(() => {
  db.run('UPDATE habits SET streak = streak + 1 WHERE id = ?', [id])
  db.run('INSERT INTO habit_log (habit_id, date) VALUES (?, date("now"))', [id])
})()
```

Данные в `@core/db`:
- Изолированы от других приложений
- Синхронизируются через CRDT на другие устройства
- Бэкапятся вместе с остальными данными Workspace
- Доступны оффлайн

### Пример: TODO-приложение без бэкенда

```typescript
import { useState, useEffect } from 'react'
import { listNotes, createNote, updateNote } from '@core/notes'
import { addTag, removeTag } from '@core/tags'

function TodoApp() {
  const [todos, setTodos] = useState([])
  const [text, setText] = useState('')

  useEffect(() => { loadTodos() }, [])

  async function loadTodos() {
    const notes = await listNotes({ tag: 'todo', project: 'current' })
    setTodos(notes)
  }

  async function addTodo() {
    const note = await createNote({ content: text, project: 'current' })
    await addTag(note.id, 'todo')
    setText('')
    loadTodos()
  }

  async function toggleDone(id, done) {
    if (done) {
      await addTag(id, 'done')
    } else {
      await removeTag(id, 'done')
    }
    loadTodos()
  }

  return (
    <div>
      <input value={text} onChange={e => setText(e.target.value)} />
      <button onClick={addTodo}>Добавить</button>
      {todos.map(t => (
        <div key={t.id} onClick={() => toggleDone(t.id, !t.tags.includes('done'))}>
          {t.tags.includes('done') ? '✓ ' : '○ '}{t.content}
        </div>
      ))}
    </div>
  )
}
```

**Что произошло:**
- Создал заметку → SQLite → CRDT → синхронизация на все устройства
- Тег `#todo` → заметка видна через поиск
- Тег `#done` → фильтрация
- Авторизация — пользователь уже в Workspace
- Бэкенд — не нужен
- Сервер — не нужен
- Бэкап — автоматический

---

## Уровень 5: «Полный натив» — 1 месяц

V8 Isolate + WebGPU. Максимальная производительность, минимальное потребление памяти. Для системных приложений и энтузиастов.

### Что доступно

Всё из уровня 4, плюс:

```typescript
// Графика — прямой доступ к WebGPU
import { drawRect, drawText, setColor, clear } from '@core/graphics'
import { createTexture, updateTexture } from '@core/graphics'
import { pushLayout, popLayout, flexColumn, flexRow } from '@core/layout'

// UI-фреймворк Workspace (встроенный)
import { Button, TextInput, List, ListItem } from '@core/ui'
import { showDialog, showToast } from '@core/ui'

// Голос
import { registerVoiceCommand } from '@core/voice'

// Низкоуровневый доступ
import { onFrame } from '@core/loop'          // requestAnimationFrame
import { getMetrics } from '@core/performance'
```

### UI-фреймворк

Workspace предоставляет встроенный UI-фреймворк — дизайн-систему с компонентами, которая рендерится через WebGPU. Приложения выглядят и ощущаются нативно.

```typescript
import { Column, Row, Text, Button, TextField, Card } from '@core/ui'
import { useState } from '@core/state'

function TodoApp() {
  const [todos, setTodos] = useState([])
  const [text, setText] = useState('')

  return Column({
    children: [
      Row({
        children: [
          TextField({ value: text, onChange: setText, placeholder: 'Новая задача' }),
          Button({ label: 'Добавить', onClick: () => addTodo(text) }),
        ],
      }),
      ...todos.map(t =>
        Card({
          child: Row({
            children: [
              Text(t.content),
              Button({ label: '✓', onClick: () => toggleDone(t.id) }),
            ],
          }),
        })
      ),
    ],
  })
}
```

### Рендеринг

```
Приложение (V8 Isolate)
  → @core/graphics API
  → Core.Graphics (Level 3)
  → WebGPU Pipeline
  → Display Server композитит с другими окнами
  → 60 FPS, без DOM, без CSS, без Chromium
```

### Manifest

```json
{
  "name": "Todo Native",
  "short_name": "Todo",
  "icon": "/icon-192.png",
  "entry": "app.ts",
  "mode": "native",
  "permissions": ["fs", "notifications", "contacts"]
}
```

`"mode": "native"` — Workspace запускает приложение в V8 Isolate (не Island Mode). Нет DOM/CSS. Рендер через @core/graphics.

### Три уровня UI-фреймворка

Разработчик выбирает, насколько глубоко погружаться в графику:

**Уровень A — Raw Graphics (`@core/graphics`)**
Прямой доступ к WebGPU/Skia-контексту. Рисуешь что хочешь: VR-интерфейс, 3D-редактор, тяжёлая игра, альтернативный рабочий стол.
- Примеры: полностью кастомный Shell, симуляция, игра.

**Уровень B — Shell API (`@core/shell`)**
Window Manager API: управление окнами, фокусом, слоями, примагничиванием. Внутри окна — полная свобода дизайна, но окна подчиняются системным правилам.
- Примеры: Spotify, Discord, VS Code — свой брендбук, но окна как у всех.

**Уровень C — Core Design System (`@core/ui`)**
Готовая библиотека компонентов: кнопки, списки, формы, блюр-панели. Рендерится через WebGPU, выглядит нативно.
- Автоматическая адаптация: `<Button>` сам подстраивается под тему («офис», «киберпанк») и устройство (палец vs мышь).
- Для 90% приложений: заметки, калькуляторы, таск-менеджеры.

#### Почему не HTML/CSS

- **DOM** — дерево из тысяч объектов. Изменил пиксель → пересчёт всей страницы (Layout Thrashing).
- **CORE** — «нарисуй прямоугольник здесь, текст — там» напрямую через WebGPU.
- В 10–20× быстрее DOM.
- Стриминг интерфейса: набор графических команд можно передать на другое устройство.

---

## Защита кода: V8 Bytecode

Разработчик может распространять приложение как **байт-код V8** — скомпилированный код без исходников. Пользователь не может прочитать или восстановить исходный код.

### Как работает

V8 при запуске JS/TS делает: `Исходный код → Парсинг → Bytecode → Выполнение`. Bun умеет сериализовать bytecode в файл — пропускает парсинг, загружает напрямую.

```bash
bun build ./server.ts --bytecode --outfile server.bun
```

Результат — файл `server.bun` с bytecode. Bun загружает его без исходного кода. Исходники — **не извлекаемы**.

### Отличие от исполняемого файла ОС

| Bun `--bytecode` | Bun `--compile` |
|-----------------|----------------|
| Байт-код V8 (~размер кода) | exe-бинарник ОС (~30-50 МБ) |
| Не содержит runtime | Содержит весь Bun runtime |
| **Платформонезависимый** | Платформозависимый |
| Загружается в V8 Isolate | Запускается как процесс ОС |

### Для каких уровней

| Уровень | Что в bytecode | Смысл |
|---------|---------------|-------|
| 1 | — | Нет. Код на сервере разработчика |
| 2 | — | Нет. Код на сервере разработчика |
| 3 | **Бэкенд** | Да. `server.bun` — bytecode, фронт — статика |
| 4 | **Весь код приложения** | Да. `app.bun` — bytecode, использует `@core/*` |
| 5 | **Весь код приложения** | Да. `app.bun` — bytecode, WebGPU натив |

### Пример: уровень 3 с bytecode

```
com.todoapp/
├── core.json
├── frontend/
│   └── index.html         ← статика
├── server.bun              ← bytecode (не исходный код!)
└── data.db
```

```json
{
  "name": "Todo App",
  "frontend": "frontend/index.html",
  "backend": "server.bun",
  "port": 8321,
  "backend_format": "bytecode"
}
```

Workspace видит `backend_format: "bytecode"` → загружает `server.bun` в V8 Isolate через Bun bytecode loader.

### Преимущества

1. **Защита кода** — исходники не извлечь (нет обратной декомпиляции в читаемый код)
2. **Кроссплатформенность** — bytecode V8 одинаковый на Windows/Linux/macOS/ARM/x64
3. **Быстрый старт** — пропускается парсинг, загрузка быстрее
4. **Малый размер** — bytecode компактнее исходников

### Ограничения

1. **Версия V8** — bytecode привязан к версии V8. При обновлении Bun старый bytecode может не работать. Решение: поле `bun_version` в манифесте, Workspace проверяет совместимость
2. **Отладка** — пользователь не может посмотреть исходный код при ошибке. Но это и цель
3. **Аудит безопасности** — Workspace не может статически проанализировать bytecode. Но sandbox ограничивает runtime-поведение

---

## Инструменты разработчика: core-dev

**core-dev** — опциональный компонент Бэка. CLI-инструмент для установки, запуска, отладки и сборки приложений. Выключен по умолчанию, включается через настройки Бэка.

### Команды

```bash
# Установить из Git (запускает как JS/TS, без компиляции)
core-dev install https://github.com/user/todoapp.git

# Установить из локальной директории (для активной разработки)
core-dev install ./my-app

# Установить из bytecode (проверить скомпилированную версию)
core-dev install ./dist/ --format bytecode

# Запустить (dev-режим, auto-reload при изменении файлов)
core-dev run com.todoapp --dev

# Отладка — логи приложения в реальном времени
core-dev logs com.todoapp --follow

# Только ошибки
core-dev logs com.todoapp --level error

# API-вызовы (что приложение запрашивает у Workspace)
core-dev logs com.todoapp --api

# CRDT-операции
core-dev logs com.todoapp --crdt

# Собрать bytecode
core-dev build com.todoapp --output ./dist/

# Собрать только бэкенд
core-dev build com.todoapp --target backend --output ./dist/server.bun

# Проверить совместимость с версией Bun
core-dev build com.todoapp --check

# Проверить манифест
core-dev validate core.json

# Показать инфо о приложении
core-dev info com.todoapp

# Публикация в App Registry (будущие версии)
core-dev publish ./dist/ --registry pkg.core.app
```

### Dev-режим (`--dev`)

Ключ `--dev` при `core-dev run`:
- **Auto-reload** — при изменении файлов в директории приложения, V8 Isolate перезапускается
- **DevTools** — F12 открывает инспектор (Island Mode: Chromium DevTools, натив: Bun inspector). Нативные приложения (`@core/ui`): `Ctrl+I` — дерево компонентов, изменение цвета/логики на лету, мгновенное применение без перезапуска
- **Verbose logs** — все API-вызовы, CRDT-операции, permissions-запросы логируются в терминал
- **Горячая перезагрузка** — изменения фронтенда без перезапуска бэкенда

### Dev-манифест

При `core-dev install ./my-app` читается `core.json` из директории. Разработчик может указать dev-адрес фронтенда:

```json
{
  "name": "Todo App (dev)",
  "frontend": "http://localhost:3000",
  "backend": "server.ts",
  "port": 8321,
  "backend_format": "script",
  "permissions": ["notifications"],
  "_dev": true
}
```

`_dev: true` — Workspace понимает что это dev-версия:
- Не кэширует агрессивно
- Показывает debug-инфо в UI (версия, формат, permissions)
- DevTools доступны без ограничений

### Интеграция с Vite/Webpack

Разработчик пишет фронт как обычно (React/Vue/Svelte). В dev-режиме Vite-сервер крутится на `localhost:3000`, Workspace открывает его в Island Mode:

```
terminal 1: cd my-app && npm run dev        ← Vite на :3000
terminal 2: core-dev run com.todoapp --dev   ← Workspace подключает :3000
```

При изменении кода — Vite hot-reload, Workspace обновляет окно.

### Workflow разработчика

```
1. Клонирование:
   core-dev install ./my-app

2. Разработка:
   core-dev run com.todoapp --dev
   → редактирует код в своём редакторе
   → auto-reload в Workspace
   → core-dev logs com.todoapp --follow

3. Проверка манифеста:
   core-dev validate core.json

4. Сборка bytecode:
   core-dev build com.todoapp --output ./dist/

5. Тест bytecode:
   core-dev install ./dist/ --format bytecode
   core-dev run com.todoapp

6. Публикация (будущие версии):
   core-dev publish ./dist/ --registry pkg.core.app
```

### Публикация — будущие версии

`core-dev publish` — загрузка приложения в App Registry (pkg.core.app). Публикация, версионирование, подпись пакетов, модерация. Подробности будут описаны в отдельном документе App Registry.

---

## Моки @core/mock — тестирование вне Workspace

npm-пакет для разработчиков, которые хотят тестировать приложения уровня 4+ вне Workspace — в обычном браузере или Node.js.

### Установка

```bash
npm install @core/mock --save-dev
```

### Что мокает

| Модуль | Что делает мок |
|--------|---------------|
| `@core/notes` | In-memory хранилище заметок (create, read, update, delete, list) |
| `@core/tags` | In-memory теги (add, remove, get, query) |
| `@core/search` | Простой поиск по мок-данным (substring match) |
| `@core/contacts` | Мок-контакты (предзаполненные, настраиваемые) |
| `@core/notifications` | `console.log` — выводит уведомления в консоль |
| `@core/db` | In-memory SQLite (реальный SQLite через `bun:sqlite` если доступен, иначе `sql.js` в браузере) |
| `@core/projects` | Мок-проекты (предзаполненные, настраиваемые) |
| `@core/tasks` | In-memory задачи |
| `@core/messenger` | `console.log` — выводит сообщения в консоль |

### Подключение

Вариант 1 — **автоматический перехват** (рекомендуется):

```typescript
// В точке входа приложения — раньше всех остальных импортов
import '@core/mock/setup'
```

`setup` перехватывает все `@core/*` импорты через import map или module resolution hook. Все API работают на мок-данных.

Вариант 2 — **явные алиасы** через bundler:

```typescript
// vite.config.ts
{
  resolve: {
    alias: {
      '@core/notes': '@core/mock/notes',
      '@core/tags': '@core/mock/tags',
      '@core/db': '@core/mock/db',
    }
  }
}
```

### Настраиваемые мок-данные

```typescript
import { mockConfig } from '@core/mock'

mockConfig.setNotes([
  { id: '1', content: 'Купить хлеб', tags: ['todo'] },
  { id: '2', content: 'Позвонить Ивану', tags: ['todo', 'urgent'] },
])

mockConfig.setContacts([
  { id: 'ivan', name: 'Иван', phone: '+79991234567' },
])

mockConfig.setProjects([
  { id: 'remont', name: 'Ремонт', tags: ['home'] },
])
```

### Определение среды в моках

`@core/mock/setup` также устанавливает `window.__WORKSPACE__` в браузере — приложение думает что оно в Workspace:

```javascript
window.__WORKSPACE__ = {
  version: '0.0.0-mock',
  level: 4,
  appId: 'dev',
  backend: null,
  db: true
}
```

### Ограничения моков

- Нет CRDT-синхронизации (данные только in-memory, при перезагрузке — сброс)
- Нет P2P (мессенджер просто логирует)
- Нет WebGPU (`@core/graphics` не мокается — для уровня 5 нужна сама Workspace)
- Нет реальной безопасности (permissions не проверяются)

---

## Обновления приложений

Уровни 1-2 — код на сервере разработчика. Залил новую версию — пользователь видит сразу. Workspace не участвует.

Уровни 3-5 — код скачивается на устройство. Нужен механизм проверки и установки обновлений.

### Кто проверяет обновления

**Workspace, не приложение.** Приложение не проверяет само — это системная функция.

1. **Бэк (фоновая задача)** — периодически опрашивает `update_url` всех установленных приложений. Если новая версия — скачивает в кэш, уведомляет Фронт
2. **Фронт (при запуске)** — если Бэк не успел проверить, Фронт спрашивает при запуске приложения. Fallback

### Версионирование в манифесте

```json
{
  "version": "1.2.0",
  "min_core_version": "1.0.0",
  "update_url": "https://pkg.core.app/com.todoapp",
  "changelog_url": "https://pkg.core.app/com.todoapp/changelog"
}
```

| Поле | Что | Обязательное |
|------|-----|-------------|
| `version` | Текущая версия (semver) | Да (уровни 3-5) |
| `min_core_version` | Минимальная версия Workspace. Если несовместимо — не обновляем | Нет |
| `update_url` | Откуда Workspace проверяет обновления | Да (уровни 3-5) |
| `changelog_url` | Что нового (показывается пользователю) | Нет |

**Источники `update_url`:**

| Источник | Формат | Для кого |
|----------|--------|----------|
| `https://pkg.core.app/...` | Официальный магазин | Публичные приложения |
| `https://api.myapp.com/core-update` | Свой сервер разработчика | Любые, полный контроль roll-out |
| `git://github.com/user/app.git` | Git-репозиторий (теги) | Open-source приложения |

### Протокол проверки обновлений

Workspace делает `GET {update_url}?version={current_version}`. Ответ:

```json
{
  "latest": "1.3.0",
  "min_core_version": "1.0.0",
  "download_url": "https://pkg.core.app/com.todoapp/1.3.0/package.tar.zst",
  "download_hash": "blake3:abc123...",
  "signature": "ed25519:xyz...",
  "changelog": "— Добавлен экспорт в PDF\n— Исправлен баг с тегами",
  "size": 2450000,
  "mandatory": false
}
```

| Поле | Что |
|------|-----|
| `latest` | Последняя версия (semver) |
| `min_core_version` | Минимальная версия Workspace для этой версии приложения |
| `download_url` | URL для скачивания пакета |
| `download_hash` | BLAKE3 хеш пакета. Workspace проверяет после скачивания |
| `signature` | Подпись разработчика (Ed25519). Workspace проверяет перед установкой |
| `changelog` | Что нового — показывается пользователю |
| `size` | Размер пакета в байтах |
| `mandatory` | Критическое обновление (security fix). Пользователь видит предупреждение |

**Если текущая версия = последняя** — сервер возвращает `304 Not Modified` (без тела).

### Git как источник обновлений

```json
{
  "update_url": "git://github.com/user/todoapp.git",
  "update_branch": "releases"
}
```

Workspace при проверке:
1. `git ls-remote {update_url} refs/tags/*` — получает список тегов
2. Находит тег с максимальной semver версией
3. Если > текущей — скачивает, проверяет `core.json` внутри
4. Обновляет

### Свой сервер обновлений

Разработчик поднимает эндпоинт:

```
GET https://api.myapp.com/core-update?version=1.2.0
→ возвращает JSON (формат выше)
```

Разработчик полностью контролирует roll-out — может выкатывать поэтапно (canary → 10% → 50% → all).

### Процесс обновления

```
1. Бэк (фоновая задача, Scheduler):
   → GET {update_url}?version={current}
   → Есть обновление? → скачивает в temp
   → Проверяет hash (BLAKE3) и signature (Ed25519)
   → Проверяет min_core_version
   → Всё ок → распаковывает в installed/com.todoapp@1.3.0/
   → Уведомляет Фронт

2. Фронт:
   → Показывает уведомление: «Todo App обновлён до 1.3.0. Добавлен экспорт в PDF»
   → При следующем запуске приложения — новая версия

3. Предыдущая версия:
   → Сохраняется в installed/com.todoapp@1.2.0/ (для отката)
   → Удаляется через N дней (default: 7) или при следующем успешном обновлении
```

### Что видит пользователь

| Ситуация | Что видит |
|----------|----------|
| Обычное обновление | Уведомление + бейдж на иконке (если закреплено). Changelog |
| `mandatory: true` | Предупреждение: «Критическое обновление безопасности» |
| Обновление при запуске | «Доступна новая версия Todo App (1.3.0). Обновить?» |
| Ошибка обновления | «Не удалось обновить Todo App. Повторим позже» |

### Настройки пользователя

| Настройка | По умолчанию |
|-----------|-------------|
| Автообновление | Вкл |
| Уведомления об обновлениях | Вкл |
| Хранение предыдущих версий | 7 дней |

### core-dev — команды для обновлений

```bash
# Проверить есть ли обновления у приложения
core-dev check-updates com.todoapp

# Принудительно обновить
core-dev update com.todoapp

# Откатить на предыдущую версию
core-dev rollback com.todoapp

# Посмотреть историю версий
core-dev versions com.todoapp
```

### Безопасность обновлений

| Вектор | Защита |
|--------|--------|
| Подмена пакета | Подпись Ed25519 + хеш BLAKE3. Без валидной подписи — установка невозможна |
| Откат на старую версию (downgrade attack) | Workspace не устанавливает версию ниже текущей (если только не `core-dev rollback`) |
| Несовместимая версия | Проверка `min_core_version` перед установкой |
| Вредоносный update_url | Разработчик указывает URL в манифесте. Пользователь видит источник при установке |

---

## Отчёты об ошибках

Разработчик хочет знать когда и почему его приложение падает. Workspace перехватывает ошибки и отправляет отчёт. Но логи не должны стать каналом утечки пользовательских данных.

### Принцип: белые списки вместо фильтрации

Workspace **не фильтрует** сырые логи (невозможно отфильтровать всё — regex не поймёт что «ООО Ромашка» — название компании, а «1234 567890» — паспорт). Вместо этого Workspace формирует отчёт из **строго определённого набора полей**, которые не могут содержать пользовательские данные по определению.

### Что перехватывает Workspace

| Источник | Что ловит |
|----------|----------|
| `uncaughtException` | Неперехваченные исключения в V8 Isolate |
| `unhandledRejection` | Неперехваченные Promise |
| Бэкенд stderr | Ошибки бэкенда (уровень 3) |
| WebGPU errors | Ошибки рендеринга (уровень 5) |

`console.error` — **не перехватывается**. Приложение может логировать что угодно в свою консоль, но это не уходит разработчику.

### Структура отчёта

Workspace формирует отчёт из белого списка полей:

```json
{
  "app_id": "com.todoapp",
  "app_version": "1.2.0",
  "workspace_version": "1.0.0",
  "timestamp": "2025-01-15T14:32:01Z",
  "error": {
    "type": "TypeError",
    "message": null,
    "stack_frames": [
      { "file": "app.js", "line": 142, "column": 15, "function": "TodoList.render" },
      { "file": "app.js", "line": 98, "column": 8, "function": "processChild" }
    ],
    "code": null
  },
  "context": {
    "route": "/project/:id",
    "action": "toggle_done",
    "is_online": true,
    "memory_mb": 45,
    "uptime_sec": 3600
  },
  "device": {
    "type": "desktop",
    "os": "windows",
    "screen": "1920x1080",
    "locale": "ru"
  }
}
```

| Поле | Что | Может содержать пользовательские данные? |
|------|-----|----------------------------------------|
| `app_id` | ID приложения из манифеста | Нет |
| `app_version` | Версия из манифеста | Нет |
| `workspace_version` | Версия Workspace | Нет |
| `timestamp` | Время ошибки (UTC) | Нет |
| `error.type` | Тип исключения (TypeError, RangeError...) | Нет |
| `error.message` | Текст ошибки | **Да — единственное рискованное поле** |
| `error.stack_frames` | file, line, column, function — без исходного кода и реальных путей | Нет |
| `error.code` | Код ошибки (если есть) | Нет |
| `context.route` | Шаблон маршрута (`/project/:id`, не реальный ID) | Нет |
| `context.action` | Последнее действие | Нет |
| `context.is_online` | Онлайн/оффлайн | Нет |
| `context.memory_mb` | Использование памяти | Нет |
| `context.uptime_sec` | Время работы приложения | Нет |
| `device.*` | Тип устройства, ОС, экран, язык | Нет |

Stack trace — **без исходного кода** и без реальных путей файловой системы. Только имя файла (из манифеста), номер строки, колонка, имя функции.

### error.message — на контроле пользователя

`error.message` — единственное поле, которое может содержать пользовательские данные. Например:

```javascript
throw new Error(`Failed to load file: ${userFileName}`)
// message: "Failed to load file: Смета_Ремонт_Иванов.pdf"
```

Workspace не пытается фильтровать message — вместо этого пользователь решает, отправлять ли его.

#### Диалог при каждой ошибке

```
┌─────────────────────────────────────────────┐
│  Ошибка в Todo App                          │
│                                             │
│  Тип: TypeError                             │
│  Место: TodoList.render, строка 142         │
│                                             │
│  Сообщение ошибки:                          │
│  ┌───────────────────────────────────────┐  │
│  │ Cannot read properties of undefined   │  │
│  │ (reading 'id')                        │  │
│  └───────────────────────────────────────┘  │
│                                             │
│  Разработчик просит разрешение отправить    │
│  текст сообщения ошибки.                    │
│                                             │
│  [Отправить с сообщением]  [Без сообщения]  │
│                                             │
│  ☐ Запретить отправку сообщений для Todo App│
└─────────────────────────────────────────────┘
```

#### Три варианта

| Действие | Что отправляется | Когда спрашиваем снова |
|----------|-----------------|----------------------|
| «Отправить с сообщением» | Структура + `message` | При следующей ошибке — спрашиваем снова |
| «Без сообщения» | Только структура (`message: null`) | При следующей ошибке — спрашиваем снова |
| Галка «Запретить всегда» | Только структура (`message: null`) | Никогда, пока пользователь не снимет запрет |

**Ключевое правило:** «Разрешить всегда» — **нет**. При каждой ошибке пользователь решает заново (если не стоит запрет). Сейчас сообщение безопасно, а через минуту — может содержать персональные данные.

#### Структура всегда отправляется

Пользователь не может отключить отправку структурированных данных — там нет персональных данных по определению. Контролируется только `message`.

### Настройки пользователя

| Настройка | Где |
|-----------|-----|
| Запретить message для приложения | В диалоге ошибки (галка «Запретить всегда») |
| Снять запрет | Command Bar → «настройки приложений» → «отчёты об ошибках» |
| Посмотреть отправленные отчёты | Command Bar → «отчёты об ошибках» |

### Защита от эксфильтрации

Разработчик не получает API для отправки произвольных данных. Нет `CoreError`, нет `reportError` — только автоматический перехват ошибок Workspace.

| Защита | Что |
|--------|-----|
| Белые списки | Только разрешённые поля, нет сырых данных |
| Нет API для разработчика | Разработчик не может отправить произвольные данные |
| Rate limiting | Max 3 отчёта/час. Больше — блокировка + предупреждение пользователю |
| Размер | Max 10 КБ на отчёт |
| Нет «разрешить всегда» для message | При каждой ошибке — вопрос заново |
| Stack trace | Только file/line/column/function, без исходного кода и реальных путей |
| Preview отчёта | Пользователь видит полный preview структуры перед отправкой |
| Mandatory HTTPS | `error_reporting_url` принимается только по HTTPS. HTTP — отклоняется |

**Угроза `Error.message`:** злоумышленное приложение может генерировать ошибки с пользовательскими данными в `message`. Rate limiting снижает, но не исключает риск. Пользователь видит preview и может отказаться от отправки. Owner видит аномалии в аудите (категория «apps»).

### Манифест

```json
{
  "error_reporting_url": "https://api.myapp.com/core-errors"
}
```

Одно поле — куда отправлять. Если не указано — отчёты не отправляются, логируются только локально.

---

## Манифест core.json — полная схема

```json
{
  "name": "string (required)",
  "short_name": "string",
  "description": "string",
  "icon": "string (path or URL)",
  "version": "string (semver, required for levels 3-5)",
  "min_core_version": "string (semver range)",
  "update_url": "string (URL or git repo, required for levels 3-5)",
  "changelog_url": "string (URL)",

  "url": "string (level 1-2: URL сайта)",
  "display": "standalone | windowed (default: windowed)",
  "permissions": ["notifications", "fullscreen"],

  "frontend": "string (level 3+: путь к index.html или .bun)",
  "frontend_format": "static | bytecode (default: static)",
  "backend": "string (level 3: путь к server.js или server.bun)",
  "backend_format": "script | bytecode (default: script)",
  "port": "number (level 3: порт для бэкенда)",

  "entry": "string (level 5: точка входа нативного приложения или .bun)",
  "entry_format": "script | bytecode (default: script)",
  "mode": "island | native (default: island)",

  "bun_version": "string (semver range, для bytecode: '>=1.0.0 <2.0.0')",

  "network_whitelist": "string[] (level 3: разрешённые домены для исходящих HTTP/HTTPS. Пустой = доступ запрещён)",

  "error_reporting_url": "string (URL, куда Workspace отправляет отчёты об ошибках)"
}
```

**Минимальные манифесты по уровням:**

```json
// Уровень 2
{ "name": "Todoist", "icon": "/icon.png", "url": "https://todoist.com", "display": "standalone" }

// Уровень 3 (исходный код)
{ "name": "Todo App", "version": "1.2.0", "icon": "/icon.png", "frontend": "dist/index.html", "backend": "server.js", "port": 8321, "network_whitelist": ["api.todoapp.com"], "update_url": "https://pkg.core.app/com.todoapp", "error_reporting_url": "https://api.myapp.com/core-errors" }

// Уровень 3 (bytecode — защита кода)
{ "name": "Todo App", "version": "1.2.0", "icon": "/icon.png", "frontend": "dist/index.html", "backend": "server.bun", "port": 8321, "backend_format": "bytecode", "network_whitelist": ["api.todoapp.com"], "update_url": "https://pkg.core.app/com.todoapp", "error_reporting_url": "https://api.myapp.com/core-errors", "bun_version": ">=1.0.0 <2.0.0" }

// Уровень 4
{ "name": "Todo Core", "version": "2.0.0", "icon": "/icon.png", "frontend": "dist/index.html", "permissions": ["notifications"], "update_url": "https://pkg.core.app/com.todocore", "error_reporting_url": "https://api.myapp.com/core-errors" }

// Уровень 5 (bytecode)
{ "name": "Todo Native", "version": "1.0.0", "icon": "/icon.png", "entry": "app.bun", "entry_format": "bytecode", "mode": "native", "permissions": ["fs", "notifications"], "update_url": "git://github.com/user/todoapp.git", "error_reporting_url": "https://api.myapp.com/core-errors", "bun_version": ">=1.0.0 <2.0.0" }
```

---

## App Registry — технически

### Хранение

```
~/.core/apps/
├── system/                        ← системные приложения (встроены)
│   ├── core-notes/
│   ├── core-calculator/
│   ├── core-player/
│   ├── core-terminal/
│   ├── core-files/
│   └── core-settings/
├── installed/                     ← установленные приложения
│   ├── com.todoapp@1.2.0/
│   │   ├── core.json
│   │   ├── frontend/
│   │   ├── backend/               ← если level 3
│   │   ├── data.db                ← app-scoped SQLite
│   │   └── cache/                 ← кэш кода
│   └── com.habittracker@2.0.1/
├── catalog/                       ← кэш каталога приложений
│   └── index.json
└── pinned/                        ← закреплённые (без установки, level 1-2)
    └── com.todoist -> https://todoist.com
```

### Установка

| Уровень | Как устанавливается |
|---------|-------------------|
| 1 | Набрал URL → закрепил. Файлы не скачиваются |
| 2 | Набрал URL → Workspace нашёл `core.json` → добавил в каталог. Файлы не скачиваются |
| 3-4 | Скачал из каталога / по адресу → код в `installed/` |
| 5 | Скачал из каталога / по адресу → код в `installed/` |

### Удаление

Удалил приложение → очистилась директория в `installed/` → не осталось ни байта. App-scoped SQLite удаляется вместе с приложением (если пользователь не выбрал «сохранить данные»).

### Магазин (pkg.core.app)

Каталог приложений. Концепция:
- Разработчик публикует приложение (level 2-5)
- Workspace индексирует `core.json` + иконку + описание
- Пользователь ищет по имени / категории
- Устанавливает → код скачивается в `installed/`
- Обновления — автоматические, через подпись (Ed25519)

Детали магазина — за рамками текущего документа.

### Корпоративное развёртывание

Админ загружает приложение в App Registry на Бэке → назначает права («всем» или «отделу X») → Фронты получают уведомление → пользователь нажимает → код загружается из Бэка, запускается.

---

## Безопасность

### Модель безопасности по уровням

| Уровень | Sandbox | Доступ к данным | Доступ к сети | Риск |
|---------|---------|----------------|--------------|------|
| 1 | Island Mode (Chromium) | Только свои cookies/storage | Свободно | Как в браузере |
| 2 | Island Mode (Chromium) | Только свои cookies/storage | Свободно | Как в браузере + notifications |
| 3 | Island Mode + V8 Isolate | App-scoped SQLite + VFS | Whitelist доменов в `core.json` | Изолированный бэкенд, ограниченная сеть |
| 4 | Island Mode | Через @core/* (capability) | Через @core/network | Только разрешённое |
| 5 | V8 Isolate (Bun) | Через fs shim (capability) | Через @core/network | Только разрешённое |

### Permissions (уровни 2-5)

Приложения запрашивают permissions в манифесте. Пользователь подтверждает при первом запуске.

| Permission | Что даёт | Уровни |
|-----------|---------|--------|
| `notifications` | Push-уведомления через Workspace | 2-5 |
| `fullscreen` | Полноэкранный режим | 2-5 |
| `fs` | Доступ к файлам через @core/fs или fs shim | 4-5 |
| `contacts` | Доступ к контактам | 4-5 |
| `network` | Сетевые запросы через @core/network | 4-5 |
| `voice` | Голосовые команды | 5 |
| `graphics` | Прямой доступ к WebGPU | 5 |

**Для уровня 3:** бэкенд работает в sandboxed V8 Isolate. Не нуждается в permissions — доступен только app-scoped SQLite и сеть. Данные Workspace (контакты, проекты) — недоступны.

### Аудит

Действия приложений логируются в аудит (категория «apps»):
- Запуск, закрытие приложения
- Запросы permissions
- API-вызовы через @core/*
- Доступ к @core/db

### App Registry Security

**Модерация и верификация:**
- **Mandatory code review + automated security scan** для всех приложений в `pkg.core.app`
- **Certificate pinning** для `pkg.core.app` и `update_url` — Workspace отказывается подключаться при несовпадении сертификата
- **Registry of developer keys** — каждое приложение подписано ключом разработчика (Ed25519). CORE Corp может отозвать скомпрометированный ключ
- **Remote app kill** — CORE Corp может пометить приложение как вредоносное; все Бэки автоматически удаляют его из `installed/`
- **Анализ suspicious permission combinations** — комбинация `fs` + `network` + `contacts` вызывает повышенный риск и требует дополнительного review

**Политика bytecode:**
- Магазин `pkg.core.app` принимает только исходный код; bytecode создаётся локально при установке
- Корпоративный Owner может запретить bytecode-приложения через `allow_bytecode_apps: false` (по умолчанию для корпоративных Бэков)
- `allow_bytecode_apps` проверяется при установке; если `false` — приложение в формате bytecode отклоняется

### core-dev Security

- **Только Owner** может включить и использовать `core-dev` на Бэке
- **Whitelist репозиториев** для корпоративных Бэков — `core-dev install` принимает только URL из разрешённого списка
- **Certificate pinning** для `update_url` — mandatory HTTPS, pinning при первой установке
- **`core-dev rollback`** — только Owner может откатить версию приложения
- **Аудит** — все действия core-dev (установка, сборка, публикация) логируются в категорию «apps»

> Подробно: модель угроз App Registry, kill switch, supply chain — в [Layer Security, §15, §18.4](layer-7-security.md).

---

## Сводная таблица

| Уровень | Рантайм | UI | Бэкенд | Данные | API доступ | Усилие разработчика |
|---------|---------|-----|--------|--------|-----------|-------------------|
| 1. «Как есть» | Island Mode (Chromium) | HTML/CSS/DOM | Свой сервер | На сервере | Нет | 0 |
| 2. «Манифест» | Island Mode (Chromium) | HTML/CSS/DOM | Свой сервер | На сервере | notifications | 5 мин |
| 3. «Бэк на месте» | Island + V8 Isolate (Bun) | HTML/CSS/DOM | Свой, локально | App-scoped SQLite | Сеть + SQLite | 1 день |
| 4. «@core/*» | Island Mode (Chromium) | HTML/CSS/DOM | Нет (Workspace Back) | Workspace VFS + @core/db | Полный @core/* | 1 неделя |
| 5. «Полный натив» | V8 Isolate (Bun) | WebGPU | Нет (Workspace Back) | Workspace VFS + @core/db | Полный + graphics | 1 месяц |

---

## Определение среды: «Я в Workspace или в браузере?»

Разработчик уровня 3 пишет один код — и для обычного браузера (удалённый сервер), и для Workspace (localhost). Нужна одна проверка, без косвенных признаков.

### Фронт (Island Mode, Chromium)

Workspace inject-ит JavaScript **до** загрузки страницы приложения:

```javascript
window.__WORKSPACE__ = {
  version: '1.0.0',
  level: 3,
  appId: 'com.todoapp',
  backend: 'http://localhost:8321',
  db: false
}
```

Фронт проверяет:

```javascript
const API = window.__WORKSPACE__?.backend ?? 'https://api.myapp.com'

fetch(`${API}/todos`)
```

В обычном браузере `window.__WORKSPACE__` не существует — `undefined`. В Workspace — объект с конфигурацией.

**Чтоinject-ится по уровням:**

| Поле | Уровень 2 | Уровень 3 | Уровень 4 |
|------|-----------|-----------|-----------|
| `version` | Да | Да | Да |
| `level` | 2 | 3 | 4 |
| `appId` | Да | Да | Да |
| `backend` | Нет | `http://localhost:PORT` | Нет |
| `db` | false | false | true |

Уровень 1 — ничего не inject-ится. Сайт работает как есть.
Уровень 5 — нет Island Mode, нет `window.__WORKSPACE__`. Используются `process.env` (см. ниже).

### Бэкенд (V8 Isolate, Bun)

Workspace подставляет env-переменные при создании V8 Isolate:

```javascript
process.env.WORKSPACE           // 'true' — мы в Workspace
process.env.workspace_version   // '1.0.0'
process.env.WORKSPACE_LEVEL     // '3'
process.env.WORKSPACE_APP_ID    // 'com.todoapp'
process.env.WORKSPACE_DB_PATH   // путь к app-scoped SQLite
```

Бэкенд проверяет:

```javascript
import { Database } from 'bun:sqlite'

let db

if (process.env.WORKSPACE === 'true') {
  db = new Database(process.env.WORKSPACE_DB_PATH)
} else {
  db = connectPostgreSQL(process.env.DATABASE_URL)
}
```

В обычном запуске (не Workspace) эти переменные не определены.

### Уровень 4: @core/* — проверка не нужна

```javascript
import { listNotes } from '@core/notes'
```

`@core/*` модули существуют только в Workspace runtime. Если импорт прошёл — мы в Workspace. Если нет — приложение уровня 4 **по определению** не работает без Workspace. Проверка не нужна.

### Уровень 5: только env

Нет Island Mode, нет `window`. Только V8 Isolate (Bun):

```javascript
if (process.env.WORKSPACE === 'true') {
  // Полный натив — доступны @core/graphics, @core/ui, @core/db
}
```

### Сводная таблица

| Кому | Механизм | Что проверяет | Уровни |
|------|---------|--------------|--------|
| Фронт (Island Mode) | `window.__WORKSPACE__` | Объектinject-ится до загрузки страницы | 2-4 |
| Бэкенд (V8 Isolate) | `process.env.WORKSPACE` | Env при создании Isolate | 3 |
| Фронт (@core/*) | `import from '@core/*'` | Модуль существует = в Workspace | 4 |
| Натив (V8 Isolate) | `process.env.WORKSPACE` | Env при создании Isolate | 5 |

---

## Связь с другими слоями

| Слой | Что описано |
|------|-------------|
| [Layer: UX](layer-1-user-experience.md) | Приложения — без установки, живут внутри проектов |
| [Layer: Фронт/Бэк](layer-3-system-split.md) | V8 Isolates, Island Mode, App Registry как компоненты |
| [Layer: Подсистемы](layer-8-technical-decomposition.md) | App Runtime, App Registry, Capability Security — технически |
| [Layer: Безопасность](layer-7-security.md) | Capability model, permissions, sandboxing по уровням приложений |

---

## Предыдущий слой

Layer 5 описывает устройства и носители — USB, диски, сеть, P2P, импорт/экспорт, принтеры. [См. layer-5-devices.md](layer-5-devices.md).

---

## Следующий слой

Layer 7 описывает безопасность — аутентификацию без паролей, шифрование, RBAC, аудит, изоляцию и модель угроз. [См. layer-7-security.md](layer-7-security.md).
