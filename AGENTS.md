# AGENTS.md — CORE OS Project Configuration

## Project: CORE OS

Распределенная, Web-native операционная система. Работает поверх любой хост-ОС (Windows/Linux/macOS/Android) как "паразит-симбионт" — забирает ресурсы, полностью заменяет пользовательский опыт.

### Documentation Structure

```
os/
├── AGENTS.md                    # Этот файл — конфигурация проекта
├── archive/                     # Исходные обсуждения (brainstorm)
│   ├── core.md                  # Основной мозговой штурм
│   ├── architector.md           # Техническая прожарка архитектором
│   ├── marketolog.md            # Маркетинговая стратегия
│   ├── investor.md              # Питч для инвестора
│   ├── gazprom.md               # Промышленный кейс (Газпром)
│   └── gorynych.md              # Консорциум Яндекс/Сбер/ВК
├── layers/                      # Слои проектирования (сверху вниз)
│   ├── layer-1-user-experience.md          # UX + Space: что видит пользователь
│   ├── layer-2-ai.md                       # AI-слой: Intent API, Voice, Generative UI, Smart Scheduler
│   ├── layer-3-system-split.md             # Фронт (Shell) и Бэк (Backoffice)
│   ├── layer-4-installation-scenarios.md   # Сценарии установки и эксплуатации
│   ├── layer-5-devices.md                  # Устройства и носители: USB, диски, сеть, P2P, принтеры
│   ├── layer-6-apps.md                     # Модель приложений: 5 уровней интеграции
│   ├── layer-7-security.md                 # Безопасность: единый кросс-слойный документ
│   ├── layer-8-technical-decomposition.md  # Подсистемы: техническая декомпозиция
│   ├── layer-9-hardware-requirements.md    # Требования к железу
│   ├── layer-10-business-model.md          # Бизнес-модель и go-to-market
│   └── layer-11-developer-reference.md     # Агрегированный справочник разработчика
├── plan/                        # План реализации: 37 этапов + roadmap
│   ├── README.md                # Принципы разделения, сводка по этапам
│   ├── roadmap.md               # Человеческое описание всех 37 этапов
│   └── phase-01..37             # Детальные спецификации этапов
└── src/                         # Исходный код (TODO)
```

### Язык общения

- **Всегда на русском** — все ответы, пояснения, обсуждения
- **Запрещены:** китайский, японский (иероглифы), украинский — нигде и никогда
- Документация: русский для проектной документации, английский для кода и коммитов

### Before Committing

1. Формат: Markdown, заголовки `##`, subsections `###`
2. Каждый документ должен быть самодостаточным — читается без остальных
3. Cross-reference: ссылаться на другие документы как `[См. layer-3-system-split.md](layer-3-system-split.md)`

### Build Commands (TODO)

```bash
# TODO: заполнить после настройки проекта
# cargo build
# bun run dev
# bun test
```
