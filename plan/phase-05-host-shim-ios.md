# Этап 5 — Host Shim: iOS

## Цель
Портировать Host Shim на iOS. После завершения этого этапа Workspace компилируется и запускается на iPhone/iPad как native iOS приложение с полноценным вводом (тач, клавиатура), аудио, хранилищем и сетью.

## Язык и стек
- **Язык:** Rust
- **Ключевые зависимости:** `winit` (бэкенд `uikit` / `appkit` для iOS), `WORKSPACE-foundation`, `WORKSPACE-graphics`, `objc` crate (для Objective-C runtime вызовов)
- **Целевая ОС:** iOS 15+ (iPhone, iPad, ARM64)

## Зависимости
- **Этап 1** — Host Shim: Windows (trait `HostBackend`, абстракции).
- **Этап 2** — Host Shim: macOS (reference реализация, iOS и macOS делят много кода).
- **Этап 3** — Host Shim: Linux (reference реализация).
- **Этап 4** — Host Shim: Android (mobile-specific patterns: touch, insets, audio focus).

## Часть системы
**Level 0 — Host Shim** [См. layer-8 §4.1, layer-4 §3.1, layer-4 §5.3]

## Требования

### 5.1 Оконная подсистема (UIKit)
- Реализация `HostBackend` для iOS через `winit` с UIKit бэкендом.
- **UIViewController:** Workspace запускается внутри `UIViewController` как full-screen view.
- **Ориентация:** portrait, landscape, auto-rotate. iPad — все ориентации. iPhone — portrait по умолчанию (настраивается).
- **Multi-window (iPad):** Stage Manager, Split View. Workspace адаптирует layout при изменении размера окна.
- **Safe Area:** учёт safe area (notch, dynamic island, home indicator). Display Server получает safe insets.
- **Status Bar:** скрыт в полноэкранном режиме WORKSPACE, показывается при свайпе сверху (системное поведение iOS).

### 5.2 Ввод с тачскрина
- **Touch:** `UITouch` events через `winit` — `Began`, `Moved`, `Ended`, `Cancelled`.
- **Multi-touch:** до 5 точек одновременно (ограничение iOS).
- **Gesture recognizers:** pinch, pan, swipe, long press, double tap — через `UIGestureRecognizer` или ручная обработка touch events.
- **Apple Pencil:** поддержка pressure, tilt, double-tap (если iPad + Pencil).
- **3D Touch / Haptic Touch:** force touch как дополнительное событие (опционально).

### 5.3 Ввод с клавиатуры
- **Software keyboard:** `UIKeyboardWillShowNotification` / `WillHideNotification`. Host Shim сообщает Display Server размер клавиатуры для inset.
- **Hardware keyboard:** Magic Keyboard / Smart Keyboard Folio. Полная поддержка раскладок, модификаторов (Command, Option, Control, Shift).
- **Cmd+Tab:** в полноэкранном режиме WORKSPACE — открывает Command Bar (как на macOS).

### 5.4 Системные жесты
- **Home Indicator (свайп снизу):** **Panic Gesture** — выход из WORKSPACE в iOS SpringBoard [См. layer-4 §5.3].
- **Control Center (свайп сверху-вправо/вниз):** работает всегда, WORKSPACE не перехватывает системные свайпы.
- **Notification Center (свайп сверху-влево/вниз):** работает всегда.
- **App Switcher (свайп снизу + удержание):** показывает iOS app switcher. WORKSPACE отображается с живым snapshot.
- **Back (свайп слева направо):** обрабатывается как `Back` или `Escape` в приложении.

### 5.5 Аудио (AVAudioEngine)
- **Capture/Playback:** через `AVAudioEngine` или `AudioUnit` (низкоуровневый). CPAL на iOS использует AudioUnit.
- **Audio Session:** настройка `AVAudioSessionCategory`:
  - `.playAndRecord` — для VoIP и голосовых команд.
  - `.playback` — для музыки/видео.
  - `.ambient` — для UI sounds (смешивается с другими приложениями).
- **Телефония:** при входящем звонке — `AVAudioSessionInterruption` → pause аудио, показать incoming call overlay.
- **AirPlay:** поддержка вывода аудио на AirPlay устройства (опционально).

### 5.6 Хранилище
- **App Sandbox:** iOS app sandbox. Workspace хранит данные в `NSDocumentDirectory` (user documents) и `NSApplicationSupportDirectory` (internal data).
- **iCloud:** интеграция с iCloud Drive для backup (опционально, через `NSFileManager` и `NSUbiquitousKeyValueStore`).
- **File Provider:** Workspace может зарегистрировать File Provider extension для доступа к VFS из Files app (опционально, post-release).

### 5.7 Сеть
- **NEPacketTunnelProvider:** для WireGuard VPN. Workspace регистрирует VPN configuration через `NETunnelProviderManager`.
- **Background modes:** `voip` background mode для поддержания P2P соединения в фоне.
- **Low Data Mode:** если включен — P2P sync только по WiFi, не по cellular.

### 5.8 Permissions
- **Runtime permissions:** iOS требует описания в `Info.plist` для каждого permission (microphone, camera, photos, location, notifications).
- **Permission dialog:** нативный iOS dialog.

### 5.9 Push Notifications (APNs)
- **Apple Push Notification service:** для remote wipe, push от Messenger когда приложение в фоне или убито.
- **Background fetch:** периодический fetch для P2P sync (ограниченный iOS, ~15 минут интервал).

### 5.10 App Lifecycle
- **Background:** при нажатии Home или swipe up — приложение уходит в background. Workspace сохраняет checkpoint (этап 14) и уменьшает FPS до 0 (pause rendering).
- **Foreground:** при возврате — восстановление checkpoint, resume rendering.
- **Termination:** при убивании системой — graceful shutdown с сохранением данных (через `applicationWillTerminate`).
- **Memory warning:** при `didReceiveMemoryWarning` — release кэшей, suspend background apps.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| UIKit launch | Запуск | Установить → WORKSPACE открывается |
| Touch | Тачскрин | Коснуться → TouchDown с координатами |
| Soft keyboard | IME | Тап в текстовое поле → клавиатура появляется |
| Home swipe | Panic Gesture | Свайп снизу → выход на SpringBoard |
| AirPods | Аудио | Воспроизведение через AirPods |
| VPN | WireGuard | Создать WG → iOS Settings показывает VPN |
| APNs token | Push | Получить APNs token → remote wipe работает |
| Background | Фон | Нажать Home → checkpoint сохранён, pause |

## Интеграция с будущими этапами
- Идентична другим Host Shim. Выход: `HostEvent` stream → этап 9 (Display Server), этап 12 (Micro-Kernel IPC).
- **Особенность:** Display Server (этап 9) должен поддерживать iOS surface (CAMetalLayer).

## Критерии приёмки
- [ ] Компилируется и запускается на iOS 15+ (iPhone и iPad).
- [ ] Touch events корректны (координаты, multi-touch до 5 точек).
- [ ] Soft keyboard появляется при фокусе, inset сообщается Display Server.
- [ ] Home swipe — выход в SpringBoard (Panic Gesture).
- [ ] Hardware keyboard (Magic Keyboard) — все клавиши работают.
- [ ] Audio: capture и playback через AVAudioEngine.
- [ ] VPN (WireGuard) активируется через NEPacketTunnelProvider.
- [ ] APNs token получен, push-уведомление доставлено.
- [ ] Background/foreground: checkpoint сохраняется и восстанавливается.

## Ссылки
- [layer-1-user-experience.md](../layers/layer-1-user-experience.md) — Окна, Panic Gesture
- [layer-4-installation-scenarios.md](../layers/layer-4-installation-scenarios.md) — iOS, выход в хост-ОС
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Host Shim §4.1
