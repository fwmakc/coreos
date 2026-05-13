# Этап 4 — Host Shim: Android

## Цель
Портировать Host Shim на Android. После завершения этого этапа Workspace компилируется и запускается на Android как Native Activity с полноценным вводом (тач, клавиатура), аудио, хранилищем и сетью.

## Язык и стек
- **Язык:** Rust
- **Ключевые зависимости:** `winit` (бэкенд `android_native_activity`), `ndk` (Native Development Kit), `jni` (для взаимодействия с Java/Kotlin layer при необходимости), `oboe` (audio, альтернатива CPAL на Android)
- **Целевая ОС:** Android 10+ (API 29+, ARM64, ARMv7, x86_64)

## Зависимости
- **Этап 1** — Host Shim: Windows (trait `HostBackend`, абстракции).
- **Этап 2** — Host Shim: macOS (reference реализация).
- **Этап 3** — Host Shim: Linux (reference реализация, Android — Linux-ядро).

## Часть системы
**Level 0 — Host Shim** [См. layer-8 §4.1, layer-4 §3.1, layer-4 §5.3]

## Требования

### 4.1 Оконная подсистема (Native Activity)
- Реализация `HostBackend` для Android через `winit` с бэкендом `android_native_activity`.
- **Surface:** `ANativeWindow` из NativeActivity, интеграция с `wgpu` surface через `raw-window-handle` (Android display handle).
- **Ориентация:** поддержка portrait, landscape, auto-rotate. Уведомление приложения об изменении ориентации.
- **Multi-window:** Android split-screen и freeform window mode. Workspace окно масштабируется корректно.
- **Insets:** обработка system bars (status bar, navigation bar, keyboard inset). Display Server получает safe area для рендеринга.

### 4.2 Ввод с тачскрина
- **Touch:** `AMOTION_EVENT_ACTION_DOWN` / `MOVE` / `UP` / `CANCEL`.
- **Multi-touch:** до 10 точек одновременно (pinch-to-zoom, rotate).
- **Gesture detection:** свайпы (вверх/вниз/влево/вправо), long press, double tap.
- **Stylus:** поддержка pressure и tilt (если устройство поддерживает).

### 4.3 Ввод с клавиатуры
- **Soft keyboard (IME):** интеграция через `AInputQueue`. При фокусе на текстовом поле — запрос показа клавиатуры. При потере фокуса — скрытие.
- **Hardware keyboard:** Bluetooth/USB клавиатура — полная поддержка раскладок.
- **Модификаторы:** Ctrl, Alt, Shift, Meta (search/button).

### 4.4 Системные кнопки
- **Назад (Back):** обрабатывается как `Escape` или специальное событие `SystemBack`. Приложение решает — обработать или передать системе.
- **Домой (Home):** **Panic Gesture** — выход из WORKSPACE в хост-ОС (Android launcher) [См. layer-4 §5.3].
- **Недавние (Recents):** отображается как стандартный Android recent apps. Workspace в recent apps показывает snapshot текущего экрана.
- **Кнопка питания:** блокировка экрана Android = блокировка экрана WORKSPACE (если WORKSPACE активно).

### 4.5 Аудио (Oboe)
- **Capture/Playback:** через `oboe` crate (AAudio, API 26+). Fallback на OpenSL ES для старых устройств.
- **Фокус аудио:** запрос `AudioFocus` при воспроизведении. При потере фокуса (звонок, другой плеер) — pause playback.
- **Телефония:** при входящем звонке — pause все аудио-приложения, показать incoming call overlay.

### 4.6 Хранилище
- **Scoped Storage:** Android 10+ scoped storage model. Workspace запрашивает `MANAGE_EXTERNAL_STORAGE` или работает в рамках scoped storage.
- **MediaStore:** интеграция с системной галереей и музыкой (опционально).
- **SharedPreferences:** для небольших настроек (fallback если нет доступа к файлам).

### 4.7 Сеть
- **VPN:** Android требует `VpnService` для создания WireGuard туннеля. Workspace регистрирует VPN service.
- **Doze mode:** в doze mode (экран выключен, устройство не заряжается) — P2P sync приостанавливается, сохраняются только push-уведомления.
- **Data Saver:** если включен Data Saver — P2P sync только по WiFi.

### 4.8 Permissions
- **Runtime permissions:** запрос разрешений при первом использовании (microphone, camera, storage, location).
- **Permission dialog:** нативный Android dialog (не кастомный, для соответствия guidelines).

### 4.9 Push Notifications (FCM)
- **Firebase Cloud Messaging:** для remote wipe, push-уведомлений от Messenger когда приложение в фоне.
- **Local notifications:** через `NotificationManager` когда WORKSPACE активно.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Native Activity | Запуск | Установить APK → WORKSPACE запускается |
| Touch | Тачскрин | Коснуться → событие TouchDown с координатами |
| Soft keyboard | IME | Тап в текстовое поле → клавиатура появляется |
| Home button | Panic Gesture | Нажать Home → выход в Android launcher |
| Back button | Назад | Нажать Back → приложение обрабатывает или выходит |
| Oboe audio | Аудио | Воспроизведение sine wave слышно |
| VPN | WireGuard | Создать WG туннель → Android показывает VPN active |
| FCM token | Push | Получить FCM token → remote wipe работает |

## Интеграция с будущими этапами
- Идентична другим Host Shim. Выход: `HostEvent` stream → этап 9 (Display Server), этап 12 (Micro-Kernel IPC).
- **Особенность:** Display Server (этап 9) должен поддерживать Android surface (ANativeWindow).

## Критерии приёмки
- [ ] APK компилируется и устанавливается на Android 10+.
- [ ] Native Activity запускается, окно открывается.
- [ ] Touch events корректны (координаты, multi-touch).
- [ ] Soft keyboard появляется при фокусе на тексте.
- [ ] Home button — выход в Android (Panic Gesture).
- [ ] Back button — обрабатывается приложением или системой.
- [ ] Oboe audio: capture и playback работают.
- [ ] VPN (WireGuard) активируется, трафик туннелируется.
- [ ] FCM token получен, push-уведомление доставлено.

## Ссылки
- [layer-1-user-experience.md](../layers/layer-1-user-experience.md) — Окна, Panic Gesture
- [layer-4-installation-scenarios.md](../layers/layer-4-installation-scenarios.md) — Android, выход в хост-ОС
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Host Shim §4.1
